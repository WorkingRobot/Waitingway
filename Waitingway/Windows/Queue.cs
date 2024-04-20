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

        var recap = Service.QueueTracker.CurrentRecap ?? throw new InvalidOperationException("Recap is null");
        var position = recap.CurrentPosition ?? throw new InvalidOperationException("Current position is null");
        var eta = recap.EstimatedEndTime - now;
        var elapsed = now - recap.StartTime;

        ImGui.TextUnformatted($"Your position: {position.PositionNumber}");
        ImGui.TextUnformatted($"Elapsed: {elapsed.ToString(Log.GetTimeSpanFormat(elapsed))}");
        if (eta.Ticks > 0 && position.PositionNumber > Config.MinimumPositionThreshold)
            ImGui.TextUnformatted($"Estimated time remaining: {eta.ToString(Log.GetTimeSpanFormat(eta))}");
        else
            ImGui.TextUnformatted("Estimated time remaining: Less than a minute");
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);
    }
}
