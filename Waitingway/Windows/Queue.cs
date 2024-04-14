using Dalamud.Interface.Windowing;
using ImGuiNET;
using System;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Waitingway.Windows;

public unsafe sealed class Queue : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags =
        ImGuiWindowFlags.NoFocusOnAppearing |
        ImGuiWindowFlags.NoDecoration |
        ImGuiWindowFlags.NoSavedSettings;

    public Queue() : base("Waitingway Queue", WindowFlags)
    {
        Size = new(600, 0);
        SizeCondition = ImGuiCond.FirstUseEver;

        SizeConstraints = new WindowSizeConstraints()
        {
            MinimumSize = new(600, 0),
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
        ImGui.Text($"{Service.QueueTracker.InQueue}");
        ImGui.Text($"{Service.QueueTracker.Position}");
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);
    }
}
