using Dalamud.Game.Addon.Events;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using Dalamud.Interface.Textures.TextureWraps;
using FFXIVClientStructs.FFXIV.Client.Graphics.Kernel;
using FFXIVClientStructs.FFXIV.Client.System.Memory;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Component.GUI;
using Lumina.Text;
using System;
using System.Collections.Generic;
using System.Linq;
using Waitingway.Utils;

namespace Waitingway.Natives;

public sealed unsafe class CharaListMenu : IDisposable
{
    private AtkUnitBase* Addon { get; set; }

    private ILoadedTextureIcon SettingsImage { get; }
    private IDalamudTextureWrap SettingsImageWrap { get; }

    private AtkComponentButton* SettingsButton { get; set; }
    private AtkUldPart* CachedPart { get; set; }
    private AtkUldAsset* CreatedAsset { get; set; }
    private AtkUldAsset* CachedAsset { get; set; }
    private void* CachedTexture { get; set; }

    private AtkTextNode* QueueSizeTextNode { get; set; }
    private AtkTextNode* QueueDurationTextNode { get; set; }

    private List<IAddonEventHandle?> EventHandles { get; } = [];

    private float? SettingsDecreasedWidth { get; set; }
    private const float SettingsPadding = 4;
    private const float Settings = 1f;

    private Api.CachedQueueEstimate? CachedQueueEstimate { get; set; }

    public CharaListMenu()
    {
        var isHr = AtkStage.Instance()->AtkTextureResourceManager->DefaultTextureVersion == 2;
        SettingsImage = IconManager.GetAssemblyTexture(isHr ? "Graphics.settings_hr1.png" : "Graphics.settings.png");
        SettingsImageWrap = SettingsImage.GetWrap();

        Service.AddonLifecycle.RegisterListener(AddonEvent.PostSetup, "_CharaSelectListMenu", OnSetup);
        Service.AddonLifecycle.RegisterListener(AddonEvent.PreFinalize, "_CharaSelectListMenu", OnFinalize);
        Service.AddonLifecycle.RegisterListener(AddonEvent.PreUpdate, "_CharaSelectListMenu", OnUpdate);
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

        Service.Api.ClearWorldQueueCache();

        // Saves an extra API call by getting all world queues at once
        var dcId = AgentLobby.Instance()->DataCenter;
        Service.Api.GetWorldQueuesCached(World.GetWorlds().Where(w => w.DatacenterId == dcId).Select(w => w.WorldId).ToArray());

        AdjustNativeUi();
    }

    private void OnFinalize(AddonEvent type, AddonArgs args)
    {
        RevertNativeUi();

        Addon = null;
    }

    private void OnUpdate(AddonEvent type, AddonArgs args)
    {
        var worldId = AgentLobby.Instance()->WorldId;
        if (worldId != 0)
        {
            CachedQueueEstimate = Service.Api.GetWorldQueuesCached(worldId)[0];
            UpdateWorldInfoTextNode(CachedQueueEstimate.Value);
        }
    }

    private void AdjustNativeUi()
    {
        var worldBtn = Addon->UldManager.SearchNodeById(4);
        var newCharaBtn = Addon->UldManager.SearchNodeById(5);
        var backupBtn = Addon->UldManager.SearchNodeById(6);
        var connectionInfoNode = Addon->UldManager.SearchNodeById(9);

        var width = MathF.Round(backupBtn->Width * Settings + SettingsPadding);
        SettingsDecreasedWidth = width;

        var settingsNode = Service.Hooks.getDuplicatedNode(&Addon->UldManager, 6, 1, 0);
        if (settingsNode == null)
            Service.Hooks.duplicateComponentNode(&Addon->UldManager, 6, 1, 0);
        settingsNode = Service.Hooks.getDuplicatedNode(&Addon->UldManager, 6, 1, 0);

        SettingsButton = settingsNode->GetAsAtkComponentButton();

        var imageNode = (AtkImageNode*)SettingsButton->GetImageNodeById(4);

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

        var tooltipHandler = WorldSelector.CreateTooltipHandler("Open Waitingway Settings"u8);
        EventHandles.AddRange([
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)settingsNode, AddonEventType.MouseOver, tooltipHandler),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)settingsNode, AddonEventType.MouseOut, tooltipHandler),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)settingsNode, AddonEventType.ButtonClick, (_, _, _) => Service.Plugin.OpenSettingsWindow())
        ]);

        GetOrCreateQueueSizeTextNode();
        GetOrCreateQueueDurationTextNode();

        AdjustWidth(worldBtn, -width / 2);
        AdjustX(newCharaBtn, -width / 2);
        AdjustWidth(newCharaBtn, -width / 2);
        AdjustX(backupBtn, -width);
        AdjustX(connectionInfoNode, width);

        settingsNode->ToggleVisibility(true);
    }

    private void RevertNativeUi()
    {
        if (SettingsDecreasedWidth is not { } width)
            return;

        SettingsDecreasedWidth = null;

        if (Addon == null)
            return;

        if (SettingsButton != null)
            SettingsButton->OwnerNode->ToggleVisibility(false);

        var worldBtn = Addon->UldManager.SearchNodeById(4);
        var newCharaBtn = Addon->UldManager.SearchNodeById(5);
        var backupBtn = Addon->UldManager.SearchNodeById(6);
        var connectionInfoNode = Addon->UldManager.SearchNodeById(9);

        AdjustWidth(worldBtn, width / 2);
        AdjustX(newCharaBtn, width / 2);
        AdjustWidth(newCharaBtn, width / 2);
        AdjustX(backupBtn, width);
        AdjustX(connectionInfoNode, -width);

        foreach (var handle in EventHandles)
        {
            if (handle != null)
                Service.AddonEventManager.RemoveEvent(handle);
        }

        CachedPart->UldAsset = CachedAsset;

        CreatedAsset->AtkTexture.KernelTexture->D3D11ShaderResourceView = CachedTexture;
        CreatedAsset->AtkTexture.ReleaseTexture();
        CreatedAsset->AtkTexture.Destroy(false);
        IMemorySpace.Free(CreatedAsset);

        if (QueueDurationTextNode != null)
        {
            var ptr = QueueDurationTextNode;

            ptr->ToggleVisibility(false);

            ptr->NextSiblingNode->PrevSiblingNode = ptr->PrevSiblingNode;

            Addon->UldManager.UpdateDrawNodeList();
            Addon->UpdateCollisionNodeList(false);

            IMemorySpace.Free(ptr);
            QueueDurationTextNode = null;
        }

        if (QueueSizeTextNode != null)
        {
            var ptr = QueueSizeTextNode;

            ptr->ToggleVisibility(false);

            ptr->NextSiblingNode->PrevSiblingNode = ptr->PrevSiblingNode;

            Addon->UldManager.UpdateDrawNodeList();
            Addon->UpdateCollisionNodeList(false);

            IMemorySpace.Free(ptr);
            QueueSizeTextNode = null;
        }
    }

    private AtkTextNode* GetOrCreateQueueSizeTextNode()
    {
        if (QueueSizeTextNode != null)
            return QueueSizeTextNode;

        var siblingTextNode = Addon->UldManager.SearchNodeById(9);

        if (siblingTextNode->PrevSiblingNode != null && siblingTextNode->PrevSiblingNode->NodeId == 5000)
            return QueueSizeTextNode = (AtkTextNode*)siblingTextNode->PrevSiblingNode;

        var textNode = IMemorySpace.GetUISpace()->Create<AtkTextNode>();
        textNode->Type = NodeType.Text;
        textNode->NodeId = 5000;
        textNode->NodeFlags = NodeFlags.AnchorTop | NodeFlags.AnchorLeft | NodeFlags.Enabled | NodeFlags.Visible | NodeFlags.EmitsEvents | NodeFlags.RespondToMouse | NodeFlags.HasCollision;
        textNode->DrawFlags = 8;
        textNode->AlignmentType = AlignmentType.Left;
        textNode->FontType = FontType.MiedingerMed;
        textNode->TextColor = new() { R = 0xFF, G = 0xFF, B = 0xFF, A = 0xFF };
        textNode->EdgeColor = new() { G = 0x99, B = 0xFF, A = 0xFF };
        textNode->BackgroundColor = new();
        textNode->TextFlags = 8;
        textNode->FontSize = 14;
        textNode->CharSpacing = 0;
        textNode->LineSpacing = 14;
        textNode->SetPositionShort(39, 84);
        textNode->SetWidth(130);
        textNode->SetHeight(14);

        var tooltipHandler = WorldSelector.CreateTooltipHandler(() =>
        {
            if (CachedQueueEstimate is not { } estimate)
                return new SeStringBuilder().ToArray();
            return WorldSelector.GetWorldNodeTooltip(estimate);
        });
        EventHandles.AddRange([
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)textNode, AddonEventType.MouseOver, tooltipHandler),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)textNode, AddonEventType.MouseOut, tooltipHandler),
        ]);

        textNode->ParentNode = siblingTextNode->ParentNode;
        textNode->PrevSiblingNode = siblingTextNode->PrevSiblingNode;
        siblingTextNode->PrevSiblingNode = (AtkResNode*)textNode;
        textNode->NextSiblingNode = siblingTextNode;

        QueueSizeTextNode = textNode;

        Addon->UldManager.UpdateDrawNodeList();
        Addon->UpdateCollisionNodeList(false);

        return textNode;
    }

    private AtkTextNode* GetOrCreateQueueDurationTextNode()
    {
        if (QueueDurationTextNode != null)
            return QueueDurationTextNode;
        
        var siblingTextNode = GetOrCreateQueueSizeTextNode();

        if (siblingTextNode->PrevSiblingNode != null && siblingTextNode->PrevSiblingNode->NodeId == 5001)
            return QueueDurationTextNode = (AtkTextNode*)siblingTextNode->PrevSiblingNode;

        var textNode = IMemorySpace.GetUISpace()->Create<AtkTextNode>();
        textNode->Type = NodeType.Text;
        textNode->NodeId = 5001;
        textNode->NodeFlags = NodeFlags.AnchorTop | NodeFlags.AnchorLeft | NodeFlags.Enabled | NodeFlags.Visible | NodeFlags.EmitsEvents | NodeFlags.RespondToMouse | NodeFlags.HasCollision;
        textNode->DrawFlags = 8;
        textNode->AlignmentType = AlignmentType.Left;
        textNode->FontType = FontType.TrumpGothic;
        textNode->TextColor = new() { R = 0xFF, G = 0xFF, B = 0xFF, A = 0xFF };
        textNode->EdgeColor = new() { G = 0x99, B = 0xFF, A = 0xFF };
        textNode->BackgroundColor = new();
        textNode->TextFlags = 8;
        textNode->FontSize = 18;
        textNode->CharSpacing = 0;
        textNode->LineSpacing = 18;
        textNode->SetPositionShort(120, 83);
        textNode->SetWidth(100);
        textNode->SetHeight(14);

        textNode->ParentNode = siblingTextNode->ParentNode;
        textNode->PrevSiblingNode = siblingTextNode->PrevSiblingNode;
        siblingTextNode->PrevSiblingNode = (AtkResNode*)textNode;
        textNode->NextSiblingNode = (AtkResNode*)siblingTextNode;

        QueueDurationTextNode = textNode;

        Addon->UldManager.UpdateDrawNodeList();

        return textNode;
    }

    private void UpdateWorldInfoTextNode(Api.CachedQueueEstimate estimate)
    {
        var sizeNode = GetOrCreateQueueSizeTextNode();
        var durationNode = GetOrCreateQueueDurationTextNode();

        switch (estimate.State)
        {
            case Api.CachedQueueEstimate.CacheState.Found:
                {
                    var pos = estimate.Estimate!.LastSize;
                    var dur = estimate.Estimate.LastDuration;

                    sizeNode->EdgeColor = WorldSelector.GetPositionColor(pos);
                    sizeNode->SetText(WorldSelector.FormatPosition(pos));

                    durationNode->ToggleVisibility(true);
                    durationNode->EdgeColor = WorldSelector.GetDurationColor(dur);
                    durationNode->SetText(WorldSelector.FormatDuration(dur));
                }
                break;
            case Api.CachedQueueEstimate.CacheState.Failed:
                sizeNode->EdgeColor = new() { R = 0xFF, G = 0x00, B = 0x00, A = 0xFF };
                sizeNode->SetText("!!!");
                durationNode->ToggleVisibility(false);
                break;
            case Api.CachedQueueEstimate.CacheState.InProgress:
                sizeNode->EdgeColor = new() { R = 0xCC, G = 0xCC, B = 0x00, A = 0xFF };
                sizeNode->SetText("...");
                durationNode->ToggleVisibility(false);
                break;
            default:
            case Api.CachedQueueEstimate.CacheState.NotFound:
                sizeNode->EdgeColor = new() { R = 0xFF, G = 0x00, B = 0x00, A = 0xFF };
                sizeNode->SetText("???");
                durationNode->ToggleVisibility(false);
                break;
        }
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
        Service.AddonLifecycle.UnregisterListener(OnFinalize);

        RevertNativeUi();

        SettingsImage.Dispose();
    }

    private static unsafe T* Calloc<T>() where T : unmanaged
    {
        var ptr = (T*)IMemorySpace.GetUISpace()->Malloc<T>();
        if (ptr == null)
            return null;

        IMemorySpace.Memset(ptr, 0, (ulong)sizeof(T));
        return ptr;
    }
}
