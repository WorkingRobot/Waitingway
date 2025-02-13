using Dalamud.Utility.Signatures;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Waitingway.Hooks;

public sealed unsafe class AtkHooks
{
    public delegate void DuplicateComponentNodeDelegate(AtkUldManager* manager, int componentNodeId, int duplicateCount, int nodeIdOffset);
    public delegate AtkResNode* GetDuplicatedNodeDelegate(AtkUldManager* manager, int nodeId, int idx, int offset);

    [Signature("E8 ?? ?? ?? ?? 40 38 7D 2C")]
    public readonly DuplicateComponentNodeDelegate duplicateComponentNode = null!;

    [Signature("E8 ?? ?? ?? ?? 45 8B DD")]
    public readonly GetDuplicatedNodeDelegate getDuplicatedNode = null!;

    public AtkHooks()
    {
        Service.GameInteropProvider.InitializeFromAttributes(this);
    }
}
