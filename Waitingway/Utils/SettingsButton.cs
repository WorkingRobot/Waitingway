using Dalamud.Game.Addon.Events;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using Dalamud.Interface.Textures.TextureWraps;
using FFXIVClientStructs.FFXIV.Client.Game.Event;
using FFXIVClientStructs.FFXIV.Client.Graphics.Kernel;
using FFXIVClientStructs.FFXIV.Client.System.Memory;
using FFXIVClientStructs.FFXIV.Component.GUI;
using System;
using System.Numerics;
using System.Reflection;

namespace Waitingway.Utils;

public sealed unsafe class SettingsButton : IDisposable
{
    private AtkUnitBase* Addon { get; set; }

    private ILoadedTextureIcon SettingsImage { get; }
    private IDalamudTextureWrap SettingsImageWrap { get; }

    private AtkComponentButton* Button { get; set; }
    private AtkUldPart* CachedPart { get; set; }
    private AtkUldAsset* CreatedAsset { get; set; }
    private AtkUldAsset* CachedAsset { get; set; }
    private void* CachedTexture { get; set; }

    private IAddonEventHandle?[]? EventHandles { get; set; }

    private Vector2 ButtonSize { get; set; }

    private float? DecreasedWidth { get; set; }
    private const float Padding = 0;
    private const float Scale = 1.1f;

    public SettingsButton()
    {
        var isHr = AtkStage.Instance()->AtkTextureResourceManager->DefaultTextureVersion == 2;
        SettingsImage = IconManager.GetAssemblyTexture(isHr ? "Graphics.settings_hr1.png" : "Graphics.settings.png");
        SettingsImageWrap = SettingsImage.GetWrap();

        Service.AddonLifecycle.RegisterListener(AddonEvent.PostSetup, "_CharaSelectListMenu", OnSetup);
        Service.AddonLifecycle.RegisterListener(AddonEvent.PreFinalize, "_CharaSelectListMenu", OnFinalize);
        var addonPtr = Service.GameGui.GetAddonByName("_CharaSelectListMenu");
        if (addonPtr != 0)
        {
            Addon = (AtkUnitBase*)addonPtr;
            AdjustNativeUi();
        }
    }

    private void OnSetup(AddonEvent type, AddonArgs args)
    {
        Addon = (AtkUnitBase*)args.Addon;

        AdjustNativeUi();
    }

    private void OnFinalize(AddonEvent type, AddonArgs args)
    {
        RevertNativeUi();

        Addon = null;
    }

    private void AdjustNativeUi()
    {
        var worldBtn = Addon->UldManager.SearchNodeById(4);
        var newCharaBtn = Addon->UldManager.SearchNodeById(5);
        var backupBtn = Addon->UldManager.SearchNodeById(6);

        var width = MathF.Round(backupBtn->Width * Scale + Padding);
        DecreasedWidth = width;

        var settingsNode = Service.Hooks.getDuplicatedNode(&Addon->UldManager, 6, 1, 0);
        if (settingsNode == null)
            Service.Hooks.duplicateComponentNode(&Addon->UldManager, 6, 1, 0);
        settingsNode = Service.Hooks.getDuplicatedNode(&Addon->UldManager, 6, 1, 0);

        Button = settingsNode->GetAsAtkComponentButton();

        var imageNode = (AtkImageNode*)Button->GetImageNodeById(4);

        CreatedAsset = Calloc<AtkUldAsset>();
        CreatedAsset->Id = 99899;
        CreatedAsset->AtkTexture.Ctor();
        CreatedAsset->AtkTexture.KernelTexture = Texture.CreateTexture2D(36, 36, 3, (uint)TextureFormat.R8G8B8A8, 0, 0);

        CachedTexture = CreatedAsset->AtkTexture.KernelTexture->D3D11ShaderResourceView;
        CreatedAsset->AtkTexture.KernelTexture->D3D11ShaderResourceView = (void*)SettingsImageWrap.ImGuiHandle;

        CreatedAsset->AtkTexture.TextureType = TextureType.KernelTexture;

        imageNode->AddRed = 0;
        imageNode->AddGreen = 0;
        imageNode->AddBlue = 0;

        imageNode->PartId = 0;
        CachedPart = imageNode->PartsList->Parts + imageNode->PartId;
        CachedAsset = CachedPart->UldAsset;
        CachedPart->UldAsset = CreatedAsset;

        EventHandles = [
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)settingsNode, AddonEventType.MouseOver, OnMouseOver),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)settingsNode, AddonEventType.MouseOut, OnMouseOut),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)settingsNode, AddonEventType.ButtonClick, OnButtonClick)
        ];
        
        AdjustWidth(worldBtn, -width / 2);
        AdjustX(newCharaBtn, -width / 2);
        AdjustWidth(newCharaBtn, -width / 2);
        AdjustX(backupBtn, -width);

        settingsNode->ToggleVisibility(true);
    }

    private static void OnMouseOver(AddonEventType type, nint atkUnitBase, nint atkResNode) =>
        AtkStage.Instance()->TooltipManager.ShowTooltip(((AtkUnitBase*)atkUnitBase)->Id, (AtkResNode*)atkResNode, "Open Waitingway Settings"u8);

    private static void OnMouseOut(AddonEventType type, nint atkUnitBase, nint atkResNode) =>
        AtkStage.Instance()->TooltipManager.HideTooltip(((AtkUnitBase*)atkUnitBase)->Id);

    private static void OnButtonClick(AddonEventType type, nint atkUnitBase, nint atkResNode) =>
        Service.Plugin.OpenSettingsWindow();

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

        AdjustWidth(worldBtn, width / 2);
        AdjustX(newCharaBtn, width / 2);
        AdjustWidth(newCharaBtn, width / 2);
        AdjustX(backupBtn, width);

        foreach (var handle in EventHandles ?? [])
        {
            if (handle != null)
                Service.AddonEventManager.RemoveEvent(handle);
        }

        if (Button == null)
            return;

        Button->OwnerNode->ToggleVisibility(false);

        CachedPart->UldAsset = CachedAsset;

        CreatedAsset->AtkTexture.KernelTexture->D3D11ShaderResourceView = CachedTexture;
        CreatedAsset->AtkTexture.ReleaseTexture();
        CreatedAsset->AtkTexture.Destroy(false);
        IMemorySpace.Free(CreatedAsset);
    }

    private void AdjustWidth(AtkResNode* node, float delta)
    {
        node->SetWidth((ushort)(node->Width + delta));

        var button = node->GetAsAtkComponentButton();
        if (button != null)
        {
            button->ButtonBGNode->SetWidth((ushort)(button->ButtonBGNode->Width + delta));
            button->ButtonTextNode->AtkResNode.SetXFloat(button->ButtonTextNode->AtkResNode.X + delta / 2f);
        }
    }

    private void AdjustX(AtkResNode* node, float delta)
    {
        node->SetXFloat(node->X + delta);
    }

    public void Dispose()
    {
        Service.AddonLifecycle.UnregisterListener(OnSetup);

        RevertNativeUi();

        SettingsImage.Dispose();
    }

    private static unsafe T* Calloc<T>() where T : unmanaged
    {
        var memspace = IMemorySpace.GetUISpace();
        var ptr = (T*)memspace->Malloc<T>();
        if (ptr == null)
            return null;

        IMemorySpace.Memset(ptr, 0, (ulong)sizeof(T));
        return ptr;
    }
}
