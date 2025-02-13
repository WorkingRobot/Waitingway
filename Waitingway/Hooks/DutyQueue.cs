using Dalamud.Game.Gui.PartyFinder.Types;
using Dalamud.Hooking;
using Dalamud.Utility.Signatures;
using FFXIVClientStructs.FFXIV.Client.Game;
using FFXIVClientStructs.FFXIV.Client.Game.Group;
using FFXIVClientStructs.FFXIV.Client.Game.UI;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Info;
using FFXIVClientStructs.FFXIV.Component.GUI;
using Lumina.Excel;
using Lumina.Excel.Sheets;
using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json.Serialization;
using Waitingway.Utils;
using static FFXIVClientStructs.FFXIV.Client.Game.UI.ContentsFinderQueueInfo;

namespace Waitingway.Hooks;

public sealed unsafe class DutyQueue : IDisposable
{
    public delegate void EnterQueueDelegate(QueueInfo queueInfo, QueueLanguage languages, RowRef<ClassJob> classJob, PartyMakeup? party);
    public delegate void PopQueueDelegate(QueueInfo queueInfo, DateTime? inProgressStartTime);
    public delegate void WithdrawQueueDelegate(RowRef<LogMessage> message);
    public delegate void UpdateQueueDelegate(BaseQueueUpdate update);
    public delegate void EnterContentDelegate(RowRef<ContentFinderCondition> content);

    public event EnterQueueDelegate? OnEnterQueue;
    public event PopQueueDelegate? OnPopQueue;
    public event WithdrawQueueDelegate? OnWithdrawQueue;
    public event UpdateQueueDelegate? OnUpdateQueue;
    public event EnterContentDelegate? OnEnterContent;

    public readonly struct ContentFlags
    {
        public required LootRuleFlags LootRule { get; init; }
        public required bool IsUnrestrictedParty { get; init; }
        public required bool IsMinIlvl { get; init; }
        public required bool IsSilenceEcho { get; init; }
        public required bool IsExplorer { get; init; }
        public required bool IsLevelSynced { get; init; }
        public required bool IsLimitedLeveling { get; init; }
        public required bool InProgressParty { get; init; }

        [SetsRequiredMembers]
        public ContentFlags(QueueFlags flags) : this()
        {
            if (flags.HasFlag(QueueFlags.GreedOnly))
                LootRule = LootRuleFlags.GreedOnly;
            else if (flags.HasFlag(QueueFlags.Lootmaster))
                LootRule = LootRuleFlags.Lootmaster;
            else
                LootRule = LootRuleFlags.None;

            if (flags.HasFlag(QueueFlags.Unrestricted))
                IsUnrestrictedParty = true;
            if (flags.HasFlag(QueueFlags.MinIlvl))
                IsMinIlvl = true;
            if (flags.HasFlag(QueueFlags.SilenceEcho))
                IsSilenceEcho = true;
            if (flags.HasFlag(QueueFlags.IsExplorer))
                IsExplorer = true;
            if (flags.HasFlag(QueueFlags.IsSynced))
                IsLevelSynced = true;
            if (flags.HasFlag(QueueFlags.LimitedLevelingRoulette))
                IsLimitedLeveling = true;
            if (flags.HasFlag(QueueFlags.RequestJoinPartyInProgress))
                InProgressParty = true;
        }
    }

    public readonly struct QueueInfo
    {
        public required RowRef[] Content { get; init; }
        public required ContentFlags Flags { get; init; }

        [SetsRequiredMembers]
        public QueueInfo(ContentsFinderUpdatePacket2* packet) : this()
        {
            Content = packet->RouletteId != 0
                ? ([(RowRef)LuminaSheets.CreateRowRef<ContentRoulette>(packet->RouletteId)])
                : new ReadOnlySpan<uint>(packet->ContentFinderConditions, 5).ToArray().Where(c => c != 0).Select(c => (RowRef)LuminaSheets.CreateRowRef<ContentFinderCondition>(c)).ToArray();

            Flags = new(packet->Flags);
        }
    }

    public readonly record struct PartyMember(byte Job, byte Level, ushort WorldId);

    public readonly struct PartyMakeup
    {
        public bool IsPartyLeader { get; }
        public PartyMember[] Members { get; }

        public PartyMakeup()
        {
            IsPartyLeader = Service.Hooks.Duty.isLocalPlayerPartyLeader() == 1;
            var currentWorld = (ushort)Service.ClientState.LocalPlayer!.CurrentWorld.RowId;
            var numberArrayPartyMember = RaptureAtkModule.Instance()->GetNumberArrayData(InfoProxyPartyMember.Instance()->NumberArrayIndex);
            HashSet<ulong> memberContentIds = [];
            List<PartyMember> members = [];
            void AddMember(ref readonly CrossRealmMember member)
            {
                if (member.ContentId == 0)
                    return;
                if (member.ClassJobId == 0 || member.Level == 0)
                    return;
                if (memberContentIds.Add(member.ContentId))
                    members.Add(new(member.ClassJobId, member.Level, (ushort)member.CurrentWorld));
            }
            void AddMemberParty(ref readonly FFXIVClientStructs.FFXIV.Client.Game.Group.PartyMember member)
            {
                if (member.ContentId == 0)
                    return;
                if (member.ClassJob == 0 || member.Level == 0)
                    return;
                if (memberContentIds.Add(member.ContentId))
                    members.Add(new(member.ClassJob, member.Level, currentWorld));
            }
            void AddMemberProxy(ref readonly InfoProxyCommonList.CharacterData member, int idx)
            {
                if (member.ContentId == 0)
                    return;
                var level = (byte)numberArrayPartyMember->IntArray[idx * 10 + 1];
                if (member.Job == 0 || level == 0)
                    return;
                if (memberContentIds.Add(member.ContentId))
                    members.Add(new(member.Job, level, currentWorld));
            }

            if (InfoProxyCrossRealm.IsAllianceRaid())
            {
                var ipcr = InfoProxyCrossRealm.Instance();
                foreach (ref var group in ipcr->CrossRealmGroups)
                {
                    foreach (ref var member in group.GroupMembers[..group.GroupMemberCount])
                        AddMember(in member);
                }
            }
            else if (InfoProxyCrossRealm.IsCrossRealmParty())
            {
                ref var group = ref InfoProxyCrossRealm.Instance()->CrossRealmGroups[0];
                foreach (ref var member in group.GroupMembers[..group.GroupMemberCount])
                    AddMember(in member);
            }
            else
            {
                var group = GroupManager.Instance()->GetGroup();
                if (group->MemberCount != 0)
                {
                    foreach (ref var member in group->PartyMembers[..group->MemberCount])
                        AddMemberParty(in member);
                }
                if (group->IsAlliance)
                {
                    foreach (ref var member in group->AllianceMembers)
                        AddMemberParty(in member);
                }

                // Not always accurate and it is cached, but I don't think we should care about this *too* much
                // TODO: Look into making a network request for this?
                var idx = 0;
                foreach (ref readonly var member in InfoProxyPartyMember.Instance()->CharDataSpan)
                    AddMemberProxy(in member, idx++);
            }

            Members = members.ToArray();
        }

        public static PartyMakeup? TryCreate()
        {
            var inParty = Service.Hooks.Duty.isLocalPlayerInParty() == 1;
            if (!inParty)
                return null;
            return new();
        }
    }

    public readonly record struct FillParam(byte Found, byte Needed);

    [JsonDerivedType(typeof(RouletteUpdate), typeDiscriminator: "roulette")]
    [JsonDerivedType(typeof(THDUpdate), typeDiscriminator: "thd")]
    [JsonDerivedType(typeof(PlayersUpdate), typeDiscriminator: "players")]
    [JsonDerivedType(typeof(WaitTimeUpdate), typeDiscriminator: "wait_time")]
    public class BaseQueueUpdate
    {
        public required DateTime Timestamp { get; init; }
        public required bool IsReservingServer { get; init; }

        public static BaseQueueUpdate Create(QueueInfoState state)
        {
            return state.ContentType switch
            {
                QueueInfoState.QueueContentType.PositionAndWaitTime => new RouletteUpdate
                {
                    Timestamp = DateTime.UtcNow,
                    RawPosition = state.PositionInQueue,
                    RawWaitTime = state.AverageWaitTime,
                    IsReservingServer = state.IsReservingServer
                },
                QueueInfoState.QueueContentType.THDAndWaitTime => new THDUpdate
                {
                    Timestamp = DateTime.UtcNow,
                    Tanks = new(state.TanksFound, state.TanksNeeded),
                    Healers = new(state.HealersFound, state.HealersNeeded),
                    DPS = new(state.DPSFound, state.DPSNeeded),
                    RawWaitTime = state.AverageWaitTime,
                    IsReservingServer = state.IsReservingServer
                },
                QueueInfoState.QueueContentType.PlayersAndWaitTime => new PlayersUpdate
                {
                    Timestamp = DateTime.UtcNow,
                    Players = new(state.PlayersFound, state.PlayersNeeded),
                    RawWaitTime = state.AverageWaitTime,
                    IsReservingServer = state.IsReservingServer
                },
                QueueInfoState.QueueContentType.WaitTime => new WaitTimeUpdate
                {
                    Timestamp = DateTime.UtcNow,
                    RawWaitTime = state.AverageWaitTime,
                    IsReservingServer = state.IsReservingServer
                },
                QueueInfoState.QueueContentType.None4 | QueueInfoState.QueueContentType.None5 => new BaseQueueUpdate
                {
                    Timestamp = DateTime.UtcNow,
                    IsReservingServer = state.IsReservingServer
                },
                _ => throw new ArgumentOutOfRangeException(nameof(state), state.ContentType, "ContentType is out of range")
            };
        }
    }

    public class WaitTimeUpdate : BaseQueueUpdate
    {
        [JsonPropertyName("wait_time")]
        public required byte RawWaitTime { get; init; }

        [JsonIgnore]
        public byte WaitTimeMinutes =>
            RawWaitTime == 0 ? (byte)30 : RawWaitTime;
    }

    public sealed class RouletteUpdate : WaitTimeUpdate
    {
        [JsonPropertyName("position")]
        public required byte RawPosition { get; init; }

        [JsonIgnore]
        public bool IsIndeterminate => RawPosition == 0;
        [JsonIgnore]
        public byte? Position => IsIndeterminate ? null : RawPosition;
    }

    public sealed class THDUpdate : WaitTimeUpdate
    {
        public required FillParam Tanks { get; init; }
        public required FillParam Healers { get; init; }
        public required FillParam DPS { get; init; }
    }

    public sealed class PlayersUpdate : WaitTimeUpdate
    {
        public required FillParam Players { get; init; }
    }

    [StructLayout(LayoutKind.Explicit, Size = 0x10)]
    public struct QueueInfoState
    {
        [FieldOffset(0x2)] public QueueContentType ContentType;
        [FieldOffset(0x3)] public bool IsReservingServer;

        // ContentType: 0
        [FieldOffset(0x4)] public byte PositionInQueue;

        // ContentType: 0-3
        [FieldOffset(0x5)] public byte AverageWaitTime;

        // ContentType: 1
        [FieldOffset(0x8)] public byte TanksFound;
        [FieldOffset(0x9)] public byte TanksNeeded;
        [FieldOffset(0xA)] public byte HealersFound;
        [FieldOffset(0xB)] public byte HealersNeeded;
        [FieldOffset(0xC)] public byte DPSFound;
        [FieldOffset(0xD)] public byte DPSNeeded;

        // ContentType: 2
        [FieldOffset(0xE)] public byte PlayersFound;
        [FieldOffset(0xF)] public byte PlayersNeeded;

        public enum QueueContentType : byte
        {
            PositionAndWaitTime = 0, // Roulette
            THDAndWaitTime = 1,
            PlayersAndWaitTime = 2,
            WaitTime = 3,
            None4 = 4,
            None5 = 5
        }
    }

    [StructLayout(LayoutKind.Explicit, Size = 0x28)]
    public struct ContentsFinderUpdatePacket2
    {
        [FieldOffset(0x00)] public QueueStates QueueState;
        [FieldOffset(0x01)] public byte ClassJobId;
        [FieldOffset(0x02)] public QueueLanguage LanguageFlags;
        [FieldOffset(0x08)] public QueueFlags Flags;
        [FieldOffset(0x10)] public byte RouletteId;
        [FieldOffset(0x13)] public bool BeganQueue;
        [FieldOffset(0x14)] public unsafe fixed uint ContentFinderConditions[5];
    }

    [Flags]
    public enum QueueLanguage : byte
    {
        Japanese = 1,
        English = 2,
        German = 4,
        French = 8
    }

    [Flags]
    public enum QueueFlags : ulong
    {
        // Queue Pop Flags
        ReqsDisabled = 0x8,
        Unrestricted = 0x2000,
        MinIlvl = 0x4000,
        GreedOnly = 0x8000,
        Lootmaster = 0x10000,
        IsSynced = 0x200000,
        LimitedLevelingRoulette = 0x400000,
        SilenceEcho = 0x10000000,
        IsExplorer = 0x100000000,
        InProgressParty = 0x80,

        // Queue Join Flags
        Unk20 = 0x20,
        Unk_A = 1 << 30,
        Unk_B = 0x20000,
        Unk_C = 0x400,
        Unk_D = 0x40,
        RequestJoinPartyInProgress = 0x2,
        Unk_F = 0x20000000,
        InitiatedByPartyMember = 0x4,
        Unk_H = 0x10
    }

    private delegate void QueueInfoProcessInfoStateDelegate(ContentsFinderQueueInfo* @this, QueueStates newState, QueueInfoState* newInfoState);
    private delegate void ProcessContentsFinderUpdatePacket2Delegate(ContentsFinderUpdatePacket2* packet);
    private delegate void QueueInfoWithdrawQueueDelegate(ContentsFinderQueueInfo* @this, uint logMessageId, ulong contentId);
    private delegate void QueueInfoDutyPopDelegate(ContentsFinderQueueInfo* @this, QueueStates newState, uint contentId, nint a4, byte isInProgressParty, byte lootRule, ulong inProgressPartyStartTimestamp, nint a8, byte isUnrestricted, byte isMinIlvl, byte isSilenceEcho, byte isExplorer, byte isSynced, byte isLimitedLeveling);
    private delegate void GameMainStartTerritoryTransitionDelegate(GameMain* a1, uint localPlayerEntityId, uint nextTerritoryTypeId, nint a4, nint a5, nint a6, ushort conditionId, nint a8, nint a9);

    [Signature("40 53 57 41 57 48 83 EC 30 0F B6 41 55", DetourName = nameof(QueueInfoProcessInfoStateDetour))]
    private readonly Hook<QueueInfoProcessInfoStateDelegate> queueInfoProcessInfoStateHook = null!;

    [Signature("48 89 5C 24 ?? 48 89 74 24 ?? 57 48 83 EC 50 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 44 24 ?? 48 8B D9 48 8D 0D", DetourName = nameof(ProcessContentsFinderUpdatePacket2Detour))]
    private readonly Hook<ProcessContentsFinderUpdatePacket2Delegate> processContentsFinderUpdatePacket2Hook = null!;

    [Signature("4C 8B DC 48 81 EC ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 84 24 ?? ?? ?? ?? C6 01 00", DetourName = nameof(QueueInfoWithdrawQueueDetour))]
    private readonly Hook<QueueInfoWithdrawQueueDelegate> queueInfoWithdrawQueueHook = null!;

    [Signature("48 89 5C 24 ?? 57 48 83 EC 20 80 79 55 00", DetourName = nameof(QueueInfoDutyPopDetour))]
    private readonly Hook<QueueInfoDutyPopDelegate> queueInfoDutyPopHook = null!;

    [Signature("E8 ?? ?? ?? ?? 45 84 E4 75 56", DetourName = nameof(GameMainStartTerritoryTransitionDetour))]
    private readonly Hook<GameMainStartTerritoryTransitionDelegate> gameMainStartTerritoryTransitionHook = null!;

    [Signature("E8 ?? ?? ?? ?? 88 9F ?? ?? ?? ?? 0F B6 F0")]
    private readonly delegate* unmanaged<byte> isLocalPlayerInParty = null!;

    [Signature("E8 ?? ?? ?? ?? 84 C0 75 3F 33 D2")]
    private readonly delegate* unmanaged<byte> isLocalPlayerPartyLeader = null!;

    public DutyQueue()
    {
        Service.GameInteropProvider.InitializeFromAttributes(this);

        queueInfoProcessInfoStateHook.Enable();
        processContentsFinderUpdatePacket2Hook.Enable();
        queueInfoWithdrawQueueHook.Enable();
        queueInfoDutyPopHook.Enable();
        gameMainStartTerritoryTransitionHook.Enable();
    }

    private void QueueInfoProcessInfoStateDetour(ContentsFinderQueueInfo* @this, QueueStates newState, QueueInfoState* newInfoState)
    {
        //Log.Debug("Packet1");
        //Log.Debug($"State: {newState}");
        //Log.Debug($"Type: {newInfoState->ContentType}");
        //// 0 = More than 30m
        //Log.Debug($"Wait: {newInfoState->AverageWaitTime}");
        //// 0 = Retrieving Info
        //// 50 = After 50 (?)
        //// vvv
        //Log.Debug($"Position: {newInfoState->PositionInQueue}");
        //Log.Debug($"T: {newInfoState->TanksFound} / {newInfoState->TanksNeeded}");
        //Log.Debug($"H: {newInfoState->HealersFound} / {newInfoState->HealersNeeded}");
        //Log.Debug($"D: {newInfoState->DPSFound} / {newInfoState->DPSNeeded}");
        //Log.Debug($"All: {newInfoState->PlayersFound} / {newInfoState->PlayersNeeded}");
        //Log.Debug($"Reserving: {newInfoState->IsReservingServer}");

        OnUpdateQueue?.Invoke(BaseQueueUpdate.Create(*newInfoState));

        queueInfoProcessInfoStateHook.Original(@this, newState, newInfoState);
    }

    private void ProcessContentsFinderUpdatePacket2Detour(ContentsFinderUpdatePacket2* packet)
    {
        //Log.Debug("Packet2");
        //Log.Debug($"State: {packet->QueueState}");
        //Log.Debug($"Lang: {packet->LanguageFlags}");
        //Log.Debug($"Began: {packet->BeganQueue}");
        //Log.Debug($"Job: {packet->ClassJobId}");
        //Log.Debug($"Flags: {packet->Flags}");
        //Log.Debug($"Roulette: {packet->RouletteId}");
        //Log.Debug($"Conditions: {string.Join(", ", new Span<int>(packet->ContentFinderConditions, 5).ToArray())}");

        if (packet->BeganQueue)
        {
            var makeup = PartyMakeup.TryCreate();
            var queueInfo = new QueueInfo(packet);
            var languages = packet->LanguageFlags;
            OnEnterQueue?.Invoke(queueInfo, languages, LuminaSheets.CreateRowRef<ClassJob>(packet->ClassJobId), makeup);
        }
        processContentsFinderUpdatePacket2Hook.Original(packet);
    }

    private void QueueInfoWithdrawQueueDetour(ContentsFinderQueueInfo* @this, uint logMessageId, ulong contentId)
    {
        //Log.Debug("Withdraw");
        //Log.Debug($"Log: {LuminaSheets.LogMessage.GetRow(logMessageId).Text.ExtractText()} ({logMessageId})");
        //Log.Debug($"Content: {contentId:X16}");

        OnWithdrawQueue?.Invoke(LuminaSheets.CreateRowRef<LogMessage>(logMessageId));
        queueInfoWithdrawQueueHook.Original(@this, logMessageId, contentId);
    }

    private void QueueInfoDutyPopDetour(ContentsFinderQueueInfo* @this, QueueStates newState, uint contentId, nint a4, byte isInProgressParty, byte lootRule, ulong inProgressPartyStartTimestamp, nint a8, byte isUnrestricted, byte isMinIlvl, byte isSilenceEcho, byte isExplorer, byte isSynced, byte isLimitedLeveling)
    {
        //Log.Debug("Duty Pop");
        //Log.Debug($"State: {newState}");
        //Log.Debug($"Content: {contentId:X8}");
        //Log.Debug($"A4: {a4:X16}");
        //Log.Debug($"A8: {a8:X16}");
        //Log.Debug($"InProgress: {isInProgressParty}");
        //Log.Debug($"Start: {inProgressPartyStartTimestamp}");
        //Log.Debug($"Loot: {lootRule}");
        //Log.Debug($"Unrestricted: {isUnrestricted}");
        //Log.Debug($"MinIlvl: {isMinIlvl}");
        //Log.Debug($"SilenceEcho: {isSilenceEcho}");
        //Log.Debug($"Explorer: {isExplorer}");
        //Log.Debug($"Synced: {isSynced}");
        //Log.Debug($"LimitedLeveling: {isLimitedLeveling}");

        var rouletteId = ContentsFinder.Instance()->QueueInfo.QueuedContentRouletteId;
        var queueInfo = new QueueInfo()
        {
            Content = rouletteId != 0
                ? ([(RowRef)LuminaSheets.CreateRowRef<ContentRoulette>(rouletteId)])
                : ([(RowRef)LuminaSheets.CreateRowRef<ContentFinderCondition>(contentId)]),
            Flags = new()
            {
                LootRule = (LootRuleFlags)lootRule,
                IsUnrestrictedParty = isUnrestricted != 0,
                IsMinIlvl = isMinIlvl != 0,
                IsSilenceEcho = isSilenceEcho != 0,
                IsExplorer = isExplorer != 0,
                IsLevelSynced = isSynced != 0,
                IsLimitedLeveling = isLimitedLeveling != 0,
                InProgressParty = isInProgressParty != 0
            }
        };
        DateTime? inProgressStartTimestamp = inProgressPartyStartTimestamp == 0 ? null : DateTimeOffset.FromUnixTimeSeconds((long)inProgressPartyStartTimestamp).UtcDateTime;
        OnPopQueue?.Invoke(queueInfo, inProgressStartTimestamp);

        queueInfoDutyPopHook.Original(@this, newState, contentId, a4, isInProgressParty, lootRule, inProgressPartyStartTimestamp, a8, isUnrestricted, isMinIlvl, isSilenceEcho, isExplorer, isSynced, isLimitedLeveling);
    }

    private void GameMainStartTerritoryTransitionDetour(GameMain* a1, uint localPlayerEntityId, uint nextTerritoryTypeId, nint a4, nint a5, nint a6, ushort conditionId, nint a8, nint a9)
    {
        //Log.Debug("Start Territory Transition");
        //Log.Debug($"LocalPlayer: {localPlayerEntityId:X8}");
        //Log.Debug($"Next Territory: {nextTerritoryTypeId}");
        //Log.Debug($"A4: {a4:X16}");
        //Log.Debug($"A5: {a5:X16}");
        //Log.Debug($"A6: {a6:X16}");
        //Log.Debug($"Condition: {conditionId:X8}");
        //Log.Debug($"A8: {a8:X16}");
        //Log.Debug($"A9: {a9:X16}");

        if (conditionId != 0)
            OnEnterContent?.Invoke(LuminaSheets.CreateRowRef<ContentFinderCondition>(conditionId));

        gameMainStartTerritoryTransitionHook.Original(a1, localPlayerEntityId, nextTerritoryTypeId, a4, a5, a6, conditionId, a8, a9);
    }

    public void Dispose()
    {
        queueInfoProcessInfoStateHook?.Dispose();
        processContentsFinderUpdatePacket2Hook?.Dispose();
        queueInfoWithdrawQueueHook?.Dispose();
        queueInfoDutyPopHook?.Dispose();
        gameMainStartTerritoryTransitionHook?.Dispose();
    }
}
