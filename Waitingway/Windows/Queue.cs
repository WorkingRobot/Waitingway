using Dalamud.Interface.Windowing;
using ImGuiNET;
using System;
using FFXIVClientStructs.FFXIV.Component.GUI;
using Waitingway.Utils;

namespace Waitingway.Windows;

public unsafe sealed class Queue : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags =
        ImGuiWindowFlags.NoFocusOnAppearing |
        ImGuiWindowFlags.NoDecoration |
        ImGuiWindowFlags.NoSavedSettings |
        ImGuiWindowFlags.AlwaysAutoResize;

    private Configuration Config => Service.Configuration;

    public Queue() : base("Waitingway Queue", WindowFlags)
    {
        Size = new(600, -1);
        SizeCondition = ImGuiCond.FirstUseEver;

        SizeConstraints = new WindowSizeConstraints()
        {
            MinimumSize = new(600, -1),
            MaximumSize = new(float.PositiveInfinity)
        };

        IsOpen = true;

        Service.WindowSystem.AddWindow(this);
    }

    public override bool DrawConditions()
    {
        if (!Service.QueueTracker.InQueue)
            return false;

        var addon = (AtkUnitBase*)Service.GameGui.GetAddonByName("SelectOk");
        if (addon == null)
            return false;

        var addon2 = (AtkUnitBase*)Service.GameGui.GetAddonByName("SelectYesno");
        if (addon2 != null)
            addon = addon2;

        if (addon->RootNode == null)
            return false;

        var x = addon->X;
        var y = addon->Y + (addon->RootNode->Height - 10) * addon->RootNode->ScaleY;
        var w = addon->RootNode->Width * addon->RootNode->ScaleX;

        Position = new(x, y);
        Size = new(w, 0);
        return true;
    }

    public override void Draw()
    {
        var now = DateTime.UtcNow;

        var startTime = Service.QueueTracker.StartTime ?? throw new InvalidOperationException("Start time is null");
        var position = Service.QueueTracker.Position ?? throw new InvalidOperationException("Position is null");
        var eta = (Service.QueueTracker.EstimateTimeRemaining(now, Config.DefaultRate, Config.Estimator switch
        {
            EstimatorType.Geometric => Estimator.GeometricWeight,
            EstimatorType.MinorGeometric => Estimator.MinorGeometricWeight,
            EstimatorType.Inverse => Estimator.InverseWeight,
            EstimatorType.ShiftedInverse => Estimator.ShiftedInverseWeight,
            _ => throw new NotSupportedException()
        }) ?? throw new InvalidOperationException("ETA is null")) - now;
        var elapsed = now - startTime;

        ImGui.TextUnformatted($"Your position: {position}");
        ImGui.TextUnformatted($"Elapsed: {elapsed.ToString(GetTimeSpanFormat(elapsed))}");
        if (eta.Ticks > 0 && position > Config.MinimumPositionThreshold)
            ImGui.TextUnformatted($"Estimated time remaining: {eta.ToString(GetTimeSpanFormat(eta))}");
        else
            ImGui.TextUnformatted("Estimated time remaining: Less than a minute");
    }

    private static string GetTimeSpanFormat(TimeSpan span)
    {
        var neg = span.Ticks < 0 ? @"\-" : string.Empty;
        var day = Math.Abs(span.TotalDays) >= 1 ? @"d\d\ " : string.Empty;
        var hour = Math.Abs(span.TotalHours) >= 1 ? @"hh\:" : string.Empty;
        return @$"{neg}{day}{hour}mm\:ss";
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);
    }
}
