using Dalamud.Interface;
using Dalamud.Interface.Utility;
using Dalamud.Interface.Utility.Raii;
using Dalamud.Interface.Windowing;
using ImGuiNET;
using System;
using System.Collections.Generic;
using System.Numerics;
using System.Threading.Tasks;
using Waitingway.Utils;

namespace Waitingway.Windows;

public sealed class Settings : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags = ImGuiWindowFlags.NoCollapse;

    private static Configuration Config => Service.Configuration;

    private const int OptionWidth = 200;
    private static Vector2 OptionButtonSize => new(OptionWidth, ImGui.GetFrameHeight());

    private string? SelectedTab { get; set; }

    private Task<Api.Connection[]>? ConnectionsTask { get; set; }
    private DateTime? ConnectionsLastRefresh { get; set; }

    private Api.Connection[]? Connections => (ConnectionsTask?.IsCompletedSuccessfully ?? false) ? ConnectionsTask.Result : null;
    private bool IsLoadingConnections => !(ConnectionsTask?.IsCompleted ?? false);
    private bool IsConnectionsUnderCooldown => ConnectionsLastRefresh is { } lastRefresh && DateTime.UtcNow - lastRefresh < TimeSpan.FromSeconds(3);

    public Settings() : base("Waitingway Settings", WindowFlags)
    {
        Service.WindowSystem.AddWindow(this);

        Size = new(600, 0);
        SizeCondition = ImGuiCond.FirstUseEver;

        SizeConstraints = new WindowSizeConstraints()
        {
            MinimumSize = new(450, 400),
            MaximumSize = new(float.PositiveInfinity)
        };
    }

    private void UpdateConnections(bool force = false)
    {
        if (IsConnectionsUnderCooldown && !force)
            return;

        ConnectionsLastRefresh = DateTime.UtcNow;
        ConnectionsTask = Service.Api.GetConnectionsAsync();
        _ = ConnectionsTask.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to load connections", "Couldn't Load Connections");
        });
    }

    public void SelectTab(string label)
    {
        SelectedTab = label;
    }

    private ImRaii.IEndObject TabItem(string label)
    {
        var isSelected = string.Equals(SelectedTab, label, StringComparison.Ordinal);
        if (isSelected)
        {
            SelectedTab = null;
            var open = true;
            return ImRaii.TabItem(label, ref open, ImGuiTabItemFlags.SetSelected);
        }
        return ImRaii.TabItem(label);
    }

    private static void DrawOption(string label, string tooltip, bool val, Action<bool> setter, ref bool isDirty)
    {
        if (ImGui.Checkbox(label, ref val))
        {
            setter(val);
            isDirty = true;
        }
        if (ImGui.IsItemHovered())
            ImGuiUtils.TooltipWrapped(tooltip);
    }

    private static void DrawOption<T>(string label, string tooltip, T value, Func<T, string> toString, Func<string, T?> fromString, Action<T> setter, ref bool isDirty)
    {
        ImGui.SetNextItemWidth(OptionWidth);
        var text = toString(value);
        if (ImGui.InputText(label, ref text, 256, ImGuiInputTextFlags.AutoSelectAll))
        {
            if (fromString(text) is { } newValue)
            {
                if (!EqualityComparer<T>.Default.Equals(value, newValue))
                {
                    setter(newValue);
                    isDirty = true;
                }
            }
        }
        if (ImGui.IsItemHovered())
            ImGuiUtils.TooltipWrapped(tooltip);
    }

    private static void DrawOption<T>(string label, string tooltip, T value, T min, T max, Action<T> setter, ref bool isDirty) where T : struct, INumber<T>
    {
        ImGui.SetNextItemWidth(OptionWidth);
        var text = value.ToString();
        if (ImGui.InputText(label, ref text, 8, ImGuiInputTextFlags.AutoSelectAll | ImGuiInputTextFlags.CharsDecimal))
        {
            if (T.TryParse(text, null, out var newValue))
            {
                newValue = T.Clamp(newValue, min, max);
                if (value != newValue)
                {
                    setter(newValue);
                    isDirty = true;
                }
            }
        }
        if (ImGui.IsItemHovered())
            ImGuiUtils.TooltipWrapped(tooltip);
    }

    private static void DrawOption<T>(string label, string tooltip, Func<T, string> getName, Func<T, string> getTooltip, T value, Action<T> setter, ref bool isDirty) where T : struct, Enum
    {
        ImGui.SetNextItemWidth(OptionWidth);
        using (var combo = ImRaii.Combo(label, getName(value)))
        {
            if (combo)
            {
                foreach (var type in Enum.GetValues<T>())
                {
                    if (ImGui.Selectable(getName(type), value.Equals(type)))
                    {
                        setter(type);
                        isDirty = true;
                    }
                    if (ImGui.IsItemHovered())
                        ImGuiUtils.TooltipWrapped(getTooltip(type));
                }
            }
        }
        if (ImGui.IsItemHovered())
            ImGuiUtils.TooltipWrapped(tooltip);
    }

    private static string GetEstimatorName(EstimatorType estimator) =>
        estimator switch
        {
            EstimatorType.Geometric => "Geometric",
            EstimatorType.MinorGeometric => "Geometric (Minor)",
            EstimatorType.Inverse => "Inverse",
            EstimatorType.ShiftedInverse => "Inverse (Shifted)",
            _ => "Unknown",
        };

    private static string GetEstimatorTooltip(EstimatorType estimator) =>
        estimator switch
        {
            EstimatorType.Geometric => "Geometric decay; 1/2",
            EstimatorType.MinorGeometric => "Geometric decay with much more emphasis on previous data points; 1/20",
            EstimatorType.Inverse => "Inverse decay; 1/n",
            EstimatorType.ShiftedInverse => "Shifted inverse decay; 1/(n+1)",
            _ => "Unknown",
        };

    public override void Draw()
    {
        if (ImGui.BeginTabBar("settingsTabBar"))
        {
            DrawTabGeneral();
            DrawTabAdvanced();
            DrawTabAbout();

            ImGui.EndTabBar();
        }
    }

    private void DrawTabGeneral()
    {
        using var tab = TabItem("General");
        if (!tab)
            return;

        ImGuiHelpers.ScaledDummy(5);

        var isDirty = false;

        if (ImGui.Button("Link Discord Account", OptionButtonSize))
        {
            var task = Service.Api.OpenConnectionLinkInBrowserAsync();
            _ = task.ContinueWith(t =>
            {
                if (t.Exception is { } e)
                    Log.ErrorNotify(e, "Failed to open Discord");
            });
        }

        ImGuiHelpers.ScaledDummy(5);

        var pos = ImGui.GetCursorPosX();
        var frameWidth = ImGui.CalcItemWidth();

        ImGui.AlignTextToFramePadding();

        ImGuiUtils.TextCentered("Connections", frameWidth);

        ImGui.SameLine();

        var buttonWidth = ImGui.GetFrameHeight();
        ImGuiUtils.AlignRight(buttonWidth, frameWidth - (ImGui.GetCursorPosX() - pos));
        var isUnderCooldown = IsConnectionsUnderCooldown;
        using (ImRaii.Disabled(isUnderCooldown))
        {
            if (ImGuiUtils.IconButtonSquare(FontAwesomeIcon.Sync, buttonWidth) || ConnectionsTask == null)
                UpdateConnections();
            if (ImGui.IsItemHovered(ImGuiHoveredFlags.AllowWhenDisabled) && isUnderCooldown)
                ImGuiUtils.TooltipWrapped("Please wait a moment before refreshing");
        }

        using (var frame = ImRaii.Child("connectionsFrame", new Vector2(frameWidth, 200), true))
        {
            if (IsLoadingConnections)
                ImGuiUtils.TextCentered("Loading...");
            else if (Connections == null)
                ImGuiUtils.TextCentered("Failed to load connections");
            else if (Connections.Length == 0)
                ImGuiUtils.TextCentered("No connections!");
            else
            {
                foreach (var connection in Connections)
                {
                    ImGui.AlignTextToFramePadding();
                    ImGui.TextUnformatted($"{connection.DisplayName} ({connection.Username})");
                    if (ImGui.IsItemHovered())
                        ImGui.SetTooltip($"ID: {connection.ConnUserId}");

                    ImGui.SameLine();
                    ImGuiUtils.AlignRight(buttonWidth);
                    if (ImGuiUtils.IconButtonSquare(FontAwesomeIcon.TrashAlt, buttonWidth))
                    {
                        var task = Service.Api.DeleteConnectionAsync(connection.ConnUserId);
                        _ = task.ContinueWith(t =>
                        {
                            if (t.Exception is { } e)
                                Log.ErrorNotify(e, "Failed to delete connection", "Couldn't Delete Connection");
                            else if (!IsLoadingConnections)
                                UpdateConnections(true);
                        });
                    }
                }
            }
        }

        if (isDirty)
            Config.Save();
    }

    private void DrawTabAdvanced()
    {
        using var tab = TabItem("Advanced");
        if (!tab)
            return;

        ImGuiHelpers.ScaledDummy(5);

        var isDirty = false;

        DrawOption(
            "Estimator",
            "The algorithm/decay function to use when estimating " +
            "the remaining time in queue.",
            GetEstimatorName,
            GetEstimatorTooltip,
            Config.Estimator,
            v => Config.Estimator = v,
            ref isDirty
        );

        DrawOption(
            "Default Queue Rate",
            "The default queue rate to use when there are not enough " +
            "data points to refer to. Units are in positions per minute.",
            Config.DefaultRate,
            1, 200,
            v => Config.DefaultRate = v,
            ref isDirty
        );

        DrawOption(
            "Minimum Position Threshold",
            "Queue positions below this level will be considered too small " +
            "to give a good estimate. If this is too low, you may see the " +
            "ETA drop into the negatives. Set to 0 to disable.",
            Config.MinimumPositionThreshold,
            0, 100,
            v => Config.MinimumPositionThreshold = v,
            ref isDirty
        );

        ImGui.Separator();

        DrawOption(
            "Notification Threshold",
            "Queue positions above this level will trigger a notification. " +
            "Keep in mind that the server also has its own threshold, so setting " +
            "this below a certain point won't have any effect.",
            Config.NotificationThreshold,
            0, 1000,
            v => Config.NotificationThreshold = v,
            ref isDirty
        );

        DrawOption(
            "Server API Url",
            "The URL of the server API to use for queue tracking. Keep this " +
            "as the default unless you're hosting a private server.",
            Config.ServerUri,
            v => v.AbsoluteUri,
            v => Uri.TryCreate(v, UriKind.Absolute, out var ret) ? ret : null,
            v => Config.ServerUri = v,
            ref isDirty
        );

        if (isDirty)
            Config.Save();
    }

    private void DrawTabAbout()
    {
        using var tab = TabItem("About");
        if (!tab)
            return;

        ImGuiHelpers.ScaledDummy(5);

        // TODO!
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);
    }
}
