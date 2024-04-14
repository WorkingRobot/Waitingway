using Dalamud.Interface.Internal;
using Dalamud.Interface.Utility.Raii;
using Dalamud.Interface.Windowing;
using FFXIVClientStructs.FFXIV.Component.GUI;
using ImGuiNET;
using System;
using System.Numerics;
using Waitingway.Utils;

namespace Waitingway.Windows;

public sealed unsafe class LobbyButton : Window, IDisposable
{
    private const ImGuiWindowFlags WindowFlags =
        ImGuiWindowFlags.AlwaysAutoResize |
        ImGuiWindowFlags.NoFocusOnAppearing |
        ImGuiWindowFlags.NoBackground |
        ImGuiWindowFlags.NoDecoration |
        ImGuiWindowFlags.NoSavedSettings;

    private AtkUnitBase* Addon { get; set; }

    private IDalamudTextureWrap SettingsImage { get; }

    public LobbyButton() : base("###Waitingway Lobby Button", WindowFlags)
    {
        SettingsImage = Service.IconManager.GetAssemblyTexture("Graphics.settings.png");

        ForceMainWindow = true;

        IsOpen = true;

        Service.WindowSystem.AddWindow(this);
    }

    public override bool DrawConditions()
    {
        Addon = (AtkUnitBase*)Service.GameGui.GetAddonByName("_CharaSelectListMenu");

        if (Addon == null)
            return false;

        // Check if addon is visible
        if (Addon->RootNode == null)
            return false;

        if (!Addon->IsVisible)
            return false;

        return true;
    }

    private Vector2 ButtonSize { get; set; }
    public override void PreDraw()
    {
        base.PreDraw();

        AdjustNativeUi();

        var backupBtn = Addon->UldManager.SearchNodeById(6);

        var x = Addon->RootNode->X + (backupBtn->X + backupBtn->Width + Padding) * Addon->RootNode->ScaleX;
        var y = Addon->RootNode->Y + (backupBtn->Y - (((Scale - 1) * backupBtn->Height) / 2)) * Addon->RootNode->ScaleY;
        var w = backupBtn->Width * Scale * Addon->RootNode->ScaleX;
        var h = backupBtn->Height * Scale * Addon->RootNode->ScaleY;

        var windowPadding = ImGui.GetStyle().WindowPadding;

        Position = new Vector2(x, y) - windowPadding;
        ButtonSize = new(w, h);
    }

    public override void PostDraw()
    {
        base.PostDraw();
    }

    private short? DecreasedWidth { get; set; }
    private float Padding => 0;
    private float Scale => 1.1f;

    private void AdjustNativeUi()
    {
        if (DecreasedWidth.HasValue)
            return;
        
        var worldBtn = Addon->UldManager.SearchNodeById(4);
        var newCharaBtn = Addon->UldManager.SearchNodeById(5);
        var backupBtn = Addon->UldManager.SearchNodeById(6);

        Log.Debug($"{newCharaBtn->X} {worldBtn->X + worldBtn->Width} {Addon->RootNode->ScaleX}");

        var width = (short)MathF.Round(backupBtn->Width * Scale + Padding);
        DecreasedWidth = width;

        AdjustWidth(worldBtn, (short)-width);
        AdjustX(newCharaBtn, (short)-width);
        AdjustX(backupBtn, (short)-width);
    }

    private void RevertNativeUi()
    {
        if (DecreasedWidth is not { } width)
            return;

        DecreasedWidth = null;

        if (Addon == null)
            return;

        var worldBtn = Addon->UldManager.SearchNodeById(4);
        var newCharaBtn = Addon->UldManager.SearchNodeById(5);
        var backupBtn = Addon->UldManager.SearchNodeById(6);

        AdjustWidth(worldBtn, width);
        AdjustX(newCharaBtn, width);
        AdjustX(backupBtn, width);
    }

    private void AdjustWidth(AtkResNode* node, short delta)
    {
        node->SetWidth((ushort)(node->Width + delta));

        var button = node->GetAsAtkComponentButton();
        if (button != null)
        {
            button->ButtonBGNode->SetWidth((ushort)(button->ButtonBGNode->Width + delta));
            button->ButtonTextNode->AtkResNode.SetX(button->ButtonTextNode->AtkResNode.X + delta / 2f);
        }
    }

    private void AdjustX(AtkResNode* node, short delta)
    {
        node->SetX((short)(node->X + delta));
    }

    public override void Draw()
    {
        ImGui.Image(SettingsImage.ImGuiHandle, ButtonSize);

        if (ImGui.IsItemHovered())
            ImGui.SetTooltip("Open Waitingway Settings");

        if (ImGui.IsItemClicked())
            Service.Plugin.OpenSettingsWindow();
    }

    public void Dispose()
    {
        Service.WindowSystem.RemoveWindow(this);

        RevertNativeUi();
    }
}
