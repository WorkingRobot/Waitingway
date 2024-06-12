using Dalamud.Interface.Windowing;
using ImGuiNET;
using System;
using FFXIVClientStructs.FFXIV.Component.GUI;
using Waitingway.Utils;

namespace Waitingway.Windows;

public sealed unsafe class Queue : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags =
        ImGuiWindowFlags.NoFocusOnAppearing |
        ImGuiWindowFlags.NoDecoration |
        ImGuiWindowFlags.NoSavedSettings |
        ImGuiWindowFlags.AlwaysAutoResize;

    private Configuration Config => Service.Configuration;

    public Queue() : base("Waitingway Queue", WindowFlags)
    {
        IsOpen = true;

        Service.WindowSystem.AddWindow(this);
    }

    public override bool DrawConditions()
    {
        if (Service.QueueTracker.CurrentState == QueueTracker.QueueState.NotQueued)
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
        switch (Service.QueueTracker.CurrentState)
        {
            case QueueTracker.QueueState.SentIdentify:
                ImGui.TextUnformatted("Status: Updating position");
                break;
            case QueueTracker.QueueState.WaitingForNextIdentify:
                if (QueueTracker.NextIdentifyTime is { } ttu && ttu > TimeSpan.Zero)
                    ImGui.TextUnformatted($"Next update in: {ttu.ToString(Log.GetTimeSpanFormat(ttu))}");
                else
                    ImGui.TextUnformatted("Next update in: Unknown");
                break;
            default:
                ImGui.TextUnformatted($"Status: {Service.QueueTracker.CurrentState}");
                break;
        }
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);
    }
}
