using Dalamud.Interface.Utility;
using Dalamud.Interface.Utility.Raii;
using Dalamud.Interface.Windowing;
using ImGuiNET;
using System;
using System.Numerics;
using Waitingway.Utils;

namespace Waitingway.Windows;

public sealed class Settings : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags = ImGuiWindowFlags.NoCollapse;

    private static Configuration Config => Service.Configuration;

    private const int OptionWidth = 200;
    private static Vector2 OptionButtonSize => new(OptionWidth, ImGui.GetFrameHeight());

    private string? SelectedTab { get; set; }

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

        DrawDiscordAccountLink();

        if (isDirty)
            Config.Save();
    }

    private void DrawDiscordAccountLink()
    {
        // TODO!
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
