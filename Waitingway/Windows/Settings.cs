using Dalamud;
using Dalamud.Bindings.ImGui;
using Dalamud.Interface;
using Dalamud.Interface.ManagedFontAtlas;
using Dalamud.Interface.Utility;
using Dalamud.Interface.Utility.Raii;
using Dalamud.Interface.Windowing;
using Dalamud.Utility.Numerics;
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Numerics;
using System.Threading.Tasks;
using Waitingway.Api.Models;
using Waitingway.Utils;

namespace Waitingway.Windows;

public sealed class Settings : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags = ImGuiWindowFlags.NoCollapse;

    private static Configuration Config => Service.Configuration;

    private const int OptionWidth = 200;
    private static Vector2 OptionButtonSize => new(OptionWidth, ImGui.GetFrameHeight());

    private string? SelectedTab { get; set; }

    private IFontHandle HeaderFont { get; }
    private IFontHandle SubheaderFont { get; }
    private IFontHandle MonoFont { get; }

    private Task<Connection[]>? ConnectionsTask { get; set; }
    private DateTime? ConnectionsLastRefresh { get; set; }

    private Connection[]? Connections => (ConnectionsTask?.IsCompletedSuccessfully ?? false) ? ConnectionsTask.Result : null;
    private bool IsLoadingConnections => !(ConnectionsTask?.IsCompleted ?? false);
    private bool IsConnectionsUnderCooldown => ConnectionsLastRefresh is { } lastRefresh && DateTime.UtcNow - lastRefresh < TimeSpan.FromSeconds(3);

    public Settings() : base("Waitingway Settings", WindowFlags)
    {
        Service.WindowSystem.AddWindow(this);

        HeaderFont = Service.PluginInterface.UiBuilder.FontAtlas.NewDelegateFontHandle(e => e.OnPreBuild(tk => tk.AddDalamudDefaultFont(UiBuilder.DefaultFontSizePx * 2f)));
        SubheaderFont = Service.PluginInterface.UiBuilder.FontAtlas.NewDelegateFontHandle(e => e.OnPreBuild(tk => tk.AddDalamudDefaultFont(UiBuilder.DefaultFontSizePx * 1.5f)));
        MonoFont = Service.PluginInterface.UiBuilder.FontAtlas.NewDelegateFontHandle(e => e.OnPreBuild(tk => tk.AddDalamudAssetFont(DalamudAsset.InconsolataRegular, new SafeFontConfig { SizePt = UiBuilder.DefaultFontSizePt * 0.9f, GlyphOffset = new Vector2(0, 1f) })));

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
        if (ImGui.InputText(label, ref text, 256, ImGuiInputTextFlags.AutoSelectAll | ImGuiInputTextFlags.EnterReturnsTrue))
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
        var text = value.ToString() ?? string.Empty;
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
            DrawTabConnections();
            DrawTabLogin();
            DrawTabDuty();
            DrawTabAdvanced();
            DrawTabAbout();

            ImGui.EndTabBar();
        }
    }

    private void DrawTabConnections()
    {
        using var tab = TabItem("Connections");
        if (!tab)
            return;

        ImGuiHelpers.ScaledDummy(5);

        var isDirty = false;

        var frameWidth = ImGui.GetContentRegionAvail().X;
        var frameHeight = ImGui.GetContentRegionAvail().Y - ImGui.GetFrameHeightWithSpacing();
        var buttonWidth = ImGui.GetFrameHeight();

        using (var frame = ImRaii.Child("connectionsFrame", new Vector2(frameWidth, frameHeight), true))
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

        var connectWidth = frameWidth - buttonWidth - ImGui.GetStyle().ItemSpacing.X;
        if (ImGui.Button("Connect Discord Account", OptionButtonSize.WithX(connectWidth)))
        {
            var task = Service.Api.OpenOAuthInBrowserAsync();
            _ = task.ContinueWith(t =>
            {
                if (t.Exception is { } e)
                    Log.ErrorNotify(e, "Failed to open Discord");
            });
        }
        if (ImGui.IsItemHovered())
            ImGuiUtils.TooltipWrapped("Connecting your Discord account will allow you to recieve queue notifications from the Waitingway discord bot.");

        ImGui.SameLine();
        var isUnderCooldown = IsConnectionsUnderCooldown;
        using (ImRaii.Disabled(isUnderCooldown))
        {
            if (ImGuiUtils.IconButtonSquare(FontAwesomeIcon.Sync, buttonWidth) || ConnectionsTask == null)
                UpdateConnections();
        }
        if (ImGui.IsItemHovered(ImGuiHoveredFlags.AllowWhenDisabled))
            ImGuiUtils.TooltipWrapped(isUnderCooldown ? "Please wait a moment before refreshing" : "Refresh connections");

        if (isDirty)
            Config.Save();
    }

    private void DrawTabLogin()
    {
        using var tab = TabItem("Login");
        if (!tab)
            return;

        ImGuiHelpers.ScaledDummy(5);

        var isDirty = false;

        DrawOption(
            "Hide Position Update Timer",
            "Hides the timer that shows what the game is doing under the " +
            "hood. Lets you know if the client is waiting to send an update " +
            "or is waiting on the server to respond.",
            Config.HideIdentifyTimer,
            v => Config.HideIdentifyTimer = v,
            ref isDirty
        );

        DrawOption(
            "Notification Threshold",
            "Only queue positions above this level will trigger a notification. " +
            "Keep in mind that the server also has its own threshold, so setting " +
            "this below a certain point won't have any effect.",
            Config.NotificationThreshold,
            0, 1000,
            v => Config.NotificationThreshold = v,
            ref isDirty
        );

        ImGui.Separator();

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
            "Position Update Latency",
            "The time it takes for the server to provide an update about " +
            "your queue positon.",
            Config.IdentifyLatency,
            0f, 3f,
            v => Config.IdentifyLatency = v,
            ref isDirty
        );

        DrawOption(
            "Login Latency",
            "The additional amount of time it takes for the server to " +
            "let you into the login process.",
            Config.LoginLatency,
            0f, 3f,
            v => Config.LoginLatency = v,
            ref isDirty
        );

        ImGui.Separator();

        DrawOption(
            "Show Queue Time in World Selector",
            "Displays the last queue time in the world select popup " +
            "instead of the last queue size.",
            Config.ShowDurationInWorldSelector,
            v => Config.ShowDurationInWorldSelector = v,
            ref isDirty
        );

        if (isDirty)
            Config.Save();
    }

    private void DrawTabDuty()
    {
        using var tab = TabItem("Duty");
        if (!tab)
            return;

        ImGuiHelpers.ScaledDummy(5);

        var isDirty = false;

        DrawOption(
            "Enable Duty Notifications",
            "Enables discord notifications for duty queues.",
            Config.DutyNotificationEnabled,
            v => Config.DutyNotificationEnabled = v,
            ref isDirty
        );

        using var _ = ImRaii.Disabled(!Config.DutyNotificationEnabled);

        DrawOption(
            "Require Confirmation",
            "Requires you to confirm before sending notifications for a " +
            "duty queue.",
            Config.DutyNotificationRequireConfirmation,
            v => Config.DutyNotificationRequireConfirmation = v,
            ref isDirty
        );

        DrawOption(
            "Allow Hidden Wait Times",
            "Allows notifications for queues with hidden wait times.",
            Config.DutyNotificationAllowHidden,
            v => Config.DutyNotificationAllowHidden = v,
            ref isDirty
        );

        DrawOption(
            "Notification Threshold (Minutes)",
            "Only queue that are estimated to be at least this many minutes will " +
            "trigger a notification. Keep in mind that the server also has its own " +
            "threshold, so setting this below a certain point won't have any effect.",
            Config.DutyNotificationThresholdMinutes,
            0, 30,
            v => Config.DutyNotificationThresholdMinutes = v,
            ref isDirty
        );

        //ImGui.Separator();

        //DrawOption(
        //    "Show Queue Info in Duty Finder",
        //    "TODO",
        //    Config.ShowDurationInWorldSelector,
        //    v => Config.ShowDurationInWorldSelector = v,
        //    ref isDirty
        //);

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
            "Server API Url",
            "The URL of the server API to use for queue tracking. Keep this " +
            "as the default unless you're hosting a private server for some " +
            "reason.",
            Config.ServerUri,
            v => v.AbsoluteUri,
            v => Uri.TryCreate(v, UriKind.Absolute, out var ret) && ret.Scheme is "http" or "https" ? ret : null,
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

        var version = Service.Version;
        var iconDim = version.Icon.Dimensions ?? new(128);

        using (var table = ImRaii.Table("settingsAboutTable", 2))
        {
            if (table)
            {
                ImGui.TableSetupColumn("", ImGuiTableColumnFlags.WidthFixed, iconDim.X);

                ImGui.TableNextColumn();
                ImGui.Image(version.Icon.ImGuiHandle, iconDim);

                ImGui.TableNextColumn();
                ImGuiUtils.AlignMiddle(new(float.PositiveInfinity, HeaderFont.GetFontSize() + SubheaderFont.GetFontSize() + ImGui.GetFontSize() * 3 + ImGui.GetStyle().ItemSpacing.Y * 4), new(0, iconDim.Y));

                using (HeaderFont.Push())
                {
                    ImGuiUtils.AlignCentered(ImGui.CalcTextSize("Waitingway").X);
                    ImGuiUtils.Hyperlink("Waitingway", "https://waiting.camora.dev", false);
                }

                using (SubheaderFont.Push())
                    ImGuiUtils.TextCentered($"v{version.VersionString} {version.BuildConfiguration}");

                ImGuiUtils.AlignCentered(ImGui.CalcTextSize($"By {version.Author} (WorkingRobot)").X);
                ImGui.Text($"By {version.Author} (");
                ImGui.SameLine(0, 0);
                ImGuiUtils.Hyperlink("WorkingRobot", "https://github.com/WorkingRobot");
                ImGui.SameLine(0, 0);
                ImGui.Text(")");

                ImGuiUtils.AlignCentered(ImGui.CalcTextSize($"Discord").X);
                ImGuiUtils.Hyperlink("Discord", "https://waiting.camora.dev/discord");

                using (ImRaii.PushColor(ImGuiCol.Text, new Vector4(0.07f, 0.76f, 1.00f, 1f)))
                {
                    ImGuiUtils.AlignCentered(ImGui.CalcTextSize($"Support me on Ko-fi!").X);
                    ImGui.TextUnformatted($"Support me on ");
                    ImGui.SameLine(0, 0);
                    ImGuiUtils.Hyperlink("Ko-fi", "https://waiting.camora.dev/funding");
                    ImGui.SameLine(0, 0);
                    ImGui.TextUnformatted("!");
                }
            }
        }

        ImGuiHelpers.ScaledDummy(5);

        ImGui.Separator();

        ImGuiHelpers.ScaledDummy(5);

        using (SubheaderFont.Push())
            ImGuiUtils.TextCentered("Server Information");

        var serverVersion = Service.Api.ServerVersion;

        ImGuiUtils.TextWrappedTo("Name: ");
        ImGui.SameLine(0, 0);
        using (MonoFont.Push())
        {
            if (serverVersion != null)
                ImGuiUtils.Hyperlink(serverVersion.Name, serverVersion.Repository, false);
            else
                ImGuiUtils.TextWrappedTo("Unknown");
        }

        if (serverVersion != null)
            ImGuiUtils.TextWrappedTo($"Version: v{serverVersion.Version} {CultureInfo.InvariantCulture.TextInfo.ToTitleCase(serverVersion.Profile)}");

        if (!string.IsNullOrWhiteSpace(serverVersion?.Description))
            ImGuiUtils.TextWrappedTo($"Description: {serverVersion.Description}");

        ImGuiHelpers.ScaledDummy(5);

        ImGui.Separator();

        ImGuiHelpers.ScaledDummy(5);

        using (SubheaderFont.Push())
            ImGuiUtils.TextCentered("Special Thanks");

        var startPosX = ImGui.GetCursorPosX();

        ImGuiUtils.TextWrappedTo("Thank you to ");
        ImGui.SameLine(0, 0);
        ImGuiUtils.Hyperlink("Lumi", "https://github.com/avafloww");
        ImGui.SameLine(0, 0);
        ImGuiUtils.TextWrappedTo(" and ");
        ImGui.SameLine(0, 0);
        ImGuiUtils.Hyperlink("NPittinger", "https://github.com/NPittinger");
        ImGui.SameLine(0, 0);
        ImGuiUtils.TextWrappedTo(" for the original Waitingway plugin");
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);

        HeaderFont.Dispose();
        SubheaderFont.Dispose();
        MonoFont.Dispose();
    }
}
