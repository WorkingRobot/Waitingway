using Dalamud.Game.Addon.Events;
using Dalamud.Game.Addon.Lifecycle;
using Dalamud.Game.Addon.Lifecycle.AddonArgTypes;
using Dalamud.Hooking;
using Dalamud.Plugin.Services;
using Dalamud.Utility.Signatures;
using FFXIVClientStructs.FFXIV.Client.Graphics;
using FFXIVClientStructs.FFXIV.Client.System.Memory;
using FFXIVClientStructs.FFXIV.Client.System.String;
using FFXIVClientStructs.FFXIV.Component.GUI;
using FFXIVClientStructs.Interop;
using Humanizer;
using Lumina.Text;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Waitingway.Api.Login.Models;
using Waitingway.Api.Models;
using Waitingway.Utils;

namespace Waitingway.Natives;

public sealed unsafe class WorldSelector : IDisposable
{
    private AddonCharaSelectWorldServer* Addon { get; set; }

    private Vector2? OldDcTextPos { get; set; }
    private Vector2? OldDcCharaIconPos { get; set; }
    private Vector2? OldDcCharaTextPos { get; set; }

    private AtkImageNode* CreatedImageNode { get; set; }
    private List<Pointer<AtkTextNode>> CreatedTextNodes { get; } = [];
    private List<IAddonEventHandle?> EventHandles { get; } = [];

    private CachedEstimate<QueueEstimate>[]? QueueEstimates { get; set; }

    [StructLayout(LayoutKind.Explicit, Size = 0x78)]
    private struct WorldEntry
    {
        [FieldOffset(0x00)] public ushort Field0;
        [FieldOffset(0x08)] public Utf8String DisplayName;
        [FieldOffset(0x70)] public ushort WorldId;
        [FieldOffset(0x74)] public uint CharacterCount;
    }

    [StructLayout(LayoutKind.Explicit, Size = 0x664)]
    private struct AddonCharaSelectWorldServer
    {
        [FieldOffset(0x000)] public AtkUnitBase Base;

        [FieldOffset(0x260)] public AtkComponentList* WorldList;
        [FieldOffset(0x278)] public WorldEntry* WorldEntriesFirst;
        [FieldOffset(0x280)] public WorldEntry* WorldEntriesEnd;
        [FieldOffset(0x288)] public WorldEntry* WorldEntriesCapacity;
    }

    private delegate void UpdateWorldEntryDelegate(AddonCharaSelectWorldServer* @this, int worldIdx, AtkResNode** nodeList, nint unk);

    [Signature("48 83 EC 38 4C 89 4C 24 ?? 4D 8B C8 44 8B C2 33 D2", DetourName = nameof(OnUpdateWorldEntry))]
    private readonly Hook<UpdateWorldEntryDelegate> updateWorldEntryHook = null!;

    public WorldSelector()
    {
        Service.GameInteropProvider.InitializeFromAttributes(this);
        updateWorldEntryHook.Enable();

        Service.AddonLifecycle.RegisterListener(AddonEvent.PostSetup, "_CharaSelectWorldServer", OnSetup);
        Service.AddonLifecycle.RegisterListener(AddonEvent.PreFinalize, "_CharaSelectWorldServer", OnFinalize);
        Service.AddonLifecycle.RegisterListener(AddonEvent.PreUpdate, "_CharaSelectWorldServer", OnUpdate);
        var addonPtr = Service.GameGui.GetAddonByName("_CharaSelectWorldServer");
        if (addonPtr != 0)
        {
            Addon = (AddonCharaSelectWorldServer*)addonPtr;
            AdjustNativeUi();
        }
    }

    private void OnSetup(AddonEvent type, AddonArgs args)
    {
        Addon = (AddonCharaSelectWorldServer*)args.Addon;

        AdjustNativeUi();
    }

    private void OnFinalize(AddonEvent type, AddonArgs args)
    {
        RevertNativeUi();

        Addon = null;
    }

    private void OnUpdate(AddonEvent type, AddonArgs args)
    {
        if (Addon == null)
            return;

        var entryCount = Addon->WorldEntriesEnd - Addon->WorldEntriesFirst;
        var entryWorldIdx = Enumerable.Range(0, (int)entryCount).Select(i => Addon->WorldEntriesFirst[i].WorldId).ToArray();

        var queues = QueueEstimates = Service.Api.Login.GetWorldQueuesCached(entryWorldIdx);
        for (var idx = 0; idx < entryCount; ++idx)
        {
            var item = Addon->WorldList->GetItemRenderer(idx);
            if (item != null)
                UpdateWorldNode(item, queues[idx]);
        }
    }

    private void OnUpdateWorldEntry(AddonCharaSelectWorldServer* @this, int worldIdx, AtkResNode** nodeList, nint unk)
    {
        updateWorldEntryHook.Original(@this, worldIdx, nodeList, unk);

        if (Addon != @this)
            return;

        var parent = nodeList[2];
        while (parent->GetAsAtkComponentNode() == null)
            parent = parent->ParentNode;
        var parentComponent = (AtkComponentListItemRenderer*)parent->GetAsAtkComponentNode()->GetComponent();

        CreateWorldNodeIfNeeded(parentComponent, worldIdx);
    }

    public static string FormatPosition(uint position) =>
        position switch
        {
            < 10000 => position.ToString(),
            < 100000 => $"{position / 1000f:.0}K",
            _ => $"{position / 1000}K"
        };

    public static string FormatDuration(TimeSpan duration)
    {
        if (duration == TimeSpan.Zero)
            return "Instant";
        if (duration > TimeSpan.FromDays(1))
            return $"{duration.TotalDays:0.0}d";
        else if (duration > TimeSpan.FromHours(1))
            return $"{duration.TotalHours:0.0}h";
        else if (duration > TimeSpan.FromMinutes(1))
            return $"{duration.TotalMinutes:0.0}m";
        else
            return $"{duration.TotalSeconds:0.0}s";
    }

    public static ByteColor GetStateColor(int state) =>
        state switch
        {
            0 => new() { R = 0x13, G = 0xFF, B = 0xA3, A = 0xFF },
            1 => new() { R = 0xCC, G = 0xCC, B = 0x00, A = 0xFF },
            2 => new() { R = 0xFF, G = 0xAA, B = 0x00, A = 0xFF },
            3 => new() { R = 0xFF, G = 0x00, B = 0x00, A = 0xFF },
            _ => new() { R = 0x00, G = 0x00, B = 0x00, A = 0x00 },
        };

    public static ByteColor GetPositionColor(uint position) =>
        GetStateColor(
            position switch
            {
                < 100 => 0,
                < 500 => 1,
                < 1000 => 2,
                _ => 3
            });

    public static ByteColor GetDurationColor(TimeSpan duration)
    {
        int state;
        if (duration < TimeSpan.FromSeconds(90))
            state = 0;
        else if (duration < TimeSpan.FromMinutes(5))
            state = 1;
        else if (duration < TimeSpan.FromMinutes(10))
            state = 2;
        else
            state = 3;
        return GetStateColor(state);
    }

    private void UpdateWorldNode(AtkComponentListItemRenderer* parentComponent, CachedEstimate<QueueEstimate> estimate)
    {
        var siblingTextNode = parentComponent->UldManager.SearchNodeById(4);

        if (siblingTextNode->PrevSiblingNode == null || siblingTextNode->PrevSiblingNode->NodeId != 5000)
            return;

        var textNode = (AtkTextNode*)siblingTextNode->PrevSiblingNode;

        ByteColor edgeColor;
        string text;

        switch (estimate.State)
        {
            case CacheState.Found:
                {
                    if (Service.Configuration.ShowDurationInWorldSelector)
                    {
                        var dur = estimate.Estimate!.LastDuration;

                        text = FormatDuration(dur);
                        edgeColor = GetDurationColor(dur);
                    }
                    else
                    {
                        var pos = estimate.Estimate!.LastSize;

                        text = FormatPosition(pos);
                        edgeColor = GetPositionColor(pos);
                    }
                }
                break;
            case CacheState.Failed:
                edgeColor = new() { R = 0xFF, G = 0x00, B = 0x00, A = 0xFF };
                text = "!!!";
                break;
            case CacheState.InProgress:
                edgeColor = new() { R = 0xCC, G = 0xCC, B = 0x00, A = 0xFF };
                text = "...";
                break;
            default:
            case CacheState.NotFound:
                edgeColor = new() { R = 0xFF, G = 0x00, B = 0x00, A = 0xFF };
                text = "???";
                break;
        }

        textNode->EdgeColor = edgeColor;
        textNode->SetText(text);
    }

    public static ReadOnlyMemory<byte> GetWorldNodeTooltip(CachedEstimate<QueueEstimate> estimate)
    {
        var b = new SeStringBuilder();
        switch (estimate.State)
        {
            case CacheState.Found:
                {
                    var pos = estimate.Estimate!.LastSize;
                    var posText = FormatPosition(pos);
                    var posColor = GetPositionColor(pos);


                    var dur = estimate.Estimate.LastDuration;
                    var durText = FormatDuration(dur);
                    var durColor = GetDurationColor(dur);

                    var up = estimate.Estimate.LastUpdate;
                    var upText = estimate.Estimate.LastUpdate.Humanize();

                    b.Append("Queue Size: ");
                    b.PushEdgeColorRgba(posColor.R, posColor.G, posColor.B, posColor.A);
                    b.Append(posText);
                    b.PopEdgeColor();
                    b.AppendNewLine();

                    b.Append("Queue Time: ");
                    b.PushEdgeColorRgba(durColor.R, durColor.G, durColor.B, durColor.A);
                    b.Append(durText);
                    b.PopEdgeColor();
                    b.AppendNewLine();

                    b.Append("Last Updated: ");
                    b.Append(upText);
                }
                break;
            case CacheState.Failed:
                b.Append("Failed to get estimate");
                break;
            case CacheState.InProgress:
                b.Append("Obtaining queue estimate...");
                break;
            default:
            case CacheState.NotFound:
                b.Append("World not found");
                break;
        }
        return b.ToArray();
    }

    private void CreateWorldNodeIfNeeded(AtkComponentListItemRenderer* parentComponent, int idx)
    {
        var parentResNode = parentComponent->UldManager.SearchNodeById(3);
        var siblingTextNode = parentComponent->UldManager.SearchNodeById(4);

        if (siblingTextNode->PrevSiblingNode != null && siblingTextNode->PrevSiblingNode->NodeId == 5000)
            return;

        var textNode = IMemorySpace.GetUISpace()->Create<AtkTextNode>();
        textNode->Type = NodeType.Text;
        textNode->NodeId = 5000;
        textNode->NodeFlags = NodeFlags.AnchorTop | NodeFlags.AnchorLeft | NodeFlags.Enabled | NodeFlags.Visible | NodeFlags.EmitsEvents | NodeFlags.RespondToMouse | NodeFlags.HasCollision;
        textNode->DrawFlags = 8;
        textNode->AlignmentType = AlignmentType.Center;
        textNode->FontType = FontType.TrumpGothic;
        textNode->TextColor = new() { R = 0xFF, G = 0xFF, B = 0xFF, A = 0xFF };
        textNode->EdgeColor = new() { G = 0x99, B = 0xFF, A = 0xFF };
        textNode->BackgroundColor = new();
        textNode->TextFlags = 8;
        textNode->FontSize = 20;
        textNode->CharSpacing = 0;
        textNode->LineSpacing = 20;
        textNode->SetPositionShort(36, 2);
        textNode->SetWidth(42);
        textNode->SetHeight(28);

        var tooltipHandler = CreateTooltipHandler(() =>
        {
            var estimates = QueueEstimates;
            if (estimates == null || estimates.Length <= idx)
                return new SeStringBuilder().ToArray();
            return GetWorldNodeTooltip(estimates[idx]);
        });
        EventHandles.AddRange([
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)textNode, AddonEventType.MouseOver, tooltipHandler),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)textNode, AddonEventType.MouseOut, tooltipHandler),
        ]);

        textNode->ParentNode = parentResNode;
        textNode->PrevSiblingNode = siblingTextNode->PrevSiblingNode;
        siblingTextNode->PrevSiblingNode = (AtkResNode*)textNode;
        textNode->NextSiblingNode = siblingTextNode;

        CreatedTextNodes.Add(textNode);

        parentComponent->UldManager.UpdateDrawNodeList();
        Addon->Base.UpdateCollisionNodeList(false);
    }

    private void CreateHeaderImageNodeIfNeeded()
    {
        var parentResNode = Addon->Base.UldManager.SearchNodeById(3);
        var parentComponent = parentResNode->GetComponent();
        var siblingNode = parentComponent->UldManager.SearchNodeById(2)->GetAsAtkImageNode();

        if ((siblingNode->PrevSiblingNode != null && siblingNode->PrevSiblingNode->NodeId == 5001) || CreatedImageNode != null)
            return;

        var imageNode = IMemorySpace.GetUISpace()->Create<AtkImageNode>();
        imageNode->Type = NodeType.Image;
        imageNode->NodeId = 5001;
        imageNode->NodeFlags = NodeFlags.AnchorTop | NodeFlags.AnchorLeft | NodeFlags.Enabled | NodeFlags.Visible | NodeFlags.EmitsEvents | NodeFlags.RespondToMouse | NodeFlags.HasCollision;
        imageNode->DrawFlags = 8;
        imageNode->WrapMode = 1;
        imageNode->PartsList = siblingNode->PartsList;
        imageNode->PartId = 26;
        imageNode->SetPositionFloat(214f, 7.5f);
        imageNode->SetWidth(36);
        imageNode->SetHeight(36);
        // 36 is the part size; 20 is the intended drawn size
        imageNode->SetScale(24 / 36f, 24 / 36f);

        var tooltipHandler = CreateTooltipHandler(() => (Service.Configuration.ShowDurationInWorldSelector ? "Queue Time"u8 : "Queue Size"u8).ToArray());
        EventHandles.AddRange([
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)imageNode, AddonEventType.MouseOver, tooltipHandler),
            Service.AddonEventManager.AddEvent((nint)Addon, (nint)imageNode, AddonEventType.MouseOut, tooltipHandler),
        ]);

        imageNode->ParentNode = parentResNode;
        imageNode->PrevSiblingNode = siblingNode->PrevSiblingNode;
        siblingNode->PrevSiblingNode = (AtkResNode*)imageNode;
        imageNode->NextSiblingNode = (AtkResNode*)siblingNode;

        CreatedImageNode = imageNode;

        parentComponent->UldManager.UpdateDrawNodeList();
        Addon->Base.UpdateCollisionNodeList(false);
    }

    private void AdjustNativeUi()
    {
        var dcButton = Addon->Base.UldManager.SearchNodeById(3)->GetAsAtkComponentButton();
        var dcText = dcButton->UldManager.SearchNodeById(3);
        var dcCharaIcon = dcButton->UldManager.SearchNodeById(2);
        var dcCharaText = dcButton->UldManager.SearchNodeById(4);

        OldDcTextPos = dcText->GetPosition();
        OldDcCharaIconPos = dcCharaIcon->GetPosition();
        OldDcCharaTextPos = dcCharaText->GetPosition();

        dcText->SetPositionFloat(18, 8.5f);
        dcCharaIcon->SetXFloat(167);
        dcCharaText->SetPositionFloat(185, 9);

        for (var idx = 0; idx < Addon->WorldList->GetItemCount(); ++idx)
        {
            var item = Addon->WorldList->GetItemRenderer(idx);
            CreateWorldNodeIfNeeded(item, idx);
        }

        CreateHeaderImageNodeIfNeeded();
    }

    public static IAddonEventManager.AddonEventHandler CreateTooltipHandler(ReadOnlySpan<byte> tooltip)
    {
        var tooltipPtr = (byte*)Unsafe.AsPointer(ref Unsafe.AsRef(in tooltip.GetPinnableReference()));
        return (type, atkUnitBase, atkResNode) =>
        {
            if (type == AddonEventType.MouseOver)
                AtkStage.Instance()->TooltipManager.ShowTooltip(((AtkUnitBase*)atkUnitBase)->Id, (AtkResNode*)atkResNode, tooltipPtr);
            else if (type == AddonEventType.MouseOut)
                AtkStage.Instance()->TooltipManager.HideTooltip(((AtkUnitBase*)atkUnitBase)->Id);
        };
    }

    public static IAddonEventManager.AddonEventHandler CreateTooltipHandler(Func<ReadOnlyMemory<byte>> tooltip)
    {
        return (type, atkUnitBase, atkResNode) =>
        {
            if (type == AddonEventType.MouseOver)
                AtkStage.Instance()->TooltipManager.ShowTooltip(((AtkUnitBase*)atkUnitBase)->Id, (AtkResNode*)atkResNode, tooltip().Span);
            else if (type == AddonEventType.MouseOut)
                AtkStage.Instance()->TooltipManager.HideTooltip(((AtkUnitBase*)atkUnitBase)->Id);
        };
    }


    private void RevertNativeUi()
    {
        if (OldDcTextPos is not { } dcTextPos)
            return;
        OldDcTextPos = null;
        if (OldDcCharaIconPos is not { } dcCharaIconPos)
            return;
        OldDcCharaIconPos = null;
        if (OldDcCharaTextPos is not { } dcCharaTextPos)
            return;

        if (Addon == null)
            return;

        var dcButton = Addon->Base.UldManager.SearchNodeById(3)->GetAsAtkComponentButton();
        var dcText = dcButton->UldManager.SearchNodeById(3);
        var dcCharaIcon = dcButton->UldManager.SearchNodeById(2);
        var dcCharaText = dcButton->UldManager.SearchNodeById(4);

        dcText->SetPosition(dcTextPos);
        dcCharaIcon->SetPosition(dcCharaIconPos);
        dcCharaText->SetPosition(dcCharaTextPos);

        foreach (var handle in EventHandles)
        {
            if (handle != null)
                Service.AddonEventManager.RemoveEvent(handle);
        }

        foreach (var node in CreatedTextNodes)
        {
            var ptr = node.Value;

            ptr->ToggleVisibility(false);

            ptr->NextSiblingNode->PrevSiblingNode = ptr->PrevSiblingNode;

            var parent = (AtkResNode*)ptr;
            while (parent->GetAsAtkComponentNode() == null)
                parent = parent->ParentNode;
            var parentComponent = (AtkComponentListItemRenderer*)parent->GetAsAtkComponentNode()->GetComponent();

            parentComponent->UldManager.UpdateDrawNodeList();

            IMemorySpace.Free(ptr);
        }
        CreatedTextNodes.Clear();

        if (CreatedImageNode != null)
        {
            if (CreatedImageNode->PrevSiblingNode != null)
                CreatedImageNode->PrevSiblingNode->NextSiblingNode = CreatedImageNode->NextSiblingNode;
            if (CreatedImageNode->NextSiblingNode != null)
                CreatedImageNode->NextSiblingNode->PrevSiblingNode = CreatedImageNode->PrevSiblingNode;

            var component = CreatedImageNode->ParentNode->GetComponent();
            if (component != null)
                component->UldManager.UpdateDrawNodeList();
            else
                Addon->Base.UldManager.UpdateDrawNodeList();
            
            IMemorySpace.Free(CreatedImageNode);

            CreatedImageNode = null;
        }

        Addon->Base.UpdateCollisionNodeList(false);
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
        updateWorldEntryHook.Dispose();

        Service.AddonLifecycle.UnregisterListener(OnSetup);
        Service.AddonLifecycle.UnregisterListener(OnFinalize);
        Service.AddonLifecycle.UnregisterListener(OnUpdate);

        RevertNativeUi();
    }
}
