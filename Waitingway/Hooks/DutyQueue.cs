using Dalamud.Game.ClientState;
using Dalamud.Game.Gui.PartyFinder.Types;
using Dalamud.Hooking;
using FFXIVClientStructs.FFXIV.Client.Enums;
using FFXIVClientStructs.FFXIV.Client.Game.Group;
using FFXIVClientStructs.FFXIV.Client.Game.Network;
using FFXIVClientStructs.FFXIV.Client.Game.UI;
using FFXIVClientStructs.FFXIV.Client.Network;
using FFXIVClientStructs.FFXIV.Client.UI;
using FFXIVClientStructs.FFXIV.Client.UI.Info;
using Lumina.Excel;
using Lumina.Excel.Sheets;
using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Linq;
using System.Text.Json.Serialization;
using Waitingway.Utils;
using static FFXIVClientStructs.FFXIV.Client.Game.Network.QueueUpdatePacket;

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
                LootRule = LootRuleFlags.Normal;

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
        public QueueInfo(QueueUpdatePacket* packet) : this()
        {
            Content = packet->RouletteId != 0
                ? [(RowRef)LuminaSheets.CreateRowRef<Lumina.Excel.Sheets.ContentRoulette>(packet->RouletteId)]
                : [.. packet->ContentFinderConditions.ToArray().Where(c => c != 0).Select(c => (RowRef)LuminaSheets.CreateRowRef<ContentFinderCondition>(c))];

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
            IsPartyLeader = InfoProxyCrossRealm.IsLocalPlayerPartyLeader();
            var currentWorld = (ushort)Service.Objects.LocalPlayer!.CurrentWorld.RowId;
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
            if (!InfoProxyCrossRealm.IsLocalPlayerInParty())
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
                QueueInfoState.QueueContentType.None4 or QueueInfoState.QueueContentType.None5 => new BaseQueueUpdate
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

    private readonly Hook<ContentsFinderQueueInfo.Delegates.ProcessInfoState> queueInfoProcessInfoStateHook = null!;

    private readonly Hook<PacketDispatcher.Delegates.HandleQueueUpdatePacket> processContentsFinderUpdatePacket2Hook = null!;

    private readonly Hook<ContentsFinderQueueInfo.Delegates.OnQueueWithdrawn> queueInfoWithdrawQueueHook = null!;

    private readonly Hook<ContentsFinderQueueInfo.Delegates.OnQueuePop> queueInfoDutyPopHook = null!;

    public DutyQueue()
    {
        queueInfoProcessInfoStateHook = Service.GameInteropProvider.HookFromAddress<ContentsFinderQueueInfo.Delegates.ProcessInfoState>((nint)ContentsFinderQueueInfo.MemberFunctionPointers.ProcessInfoState, QueueInfoProcessInfoStateDetour);
        processContentsFinderUpdatePacket2Hook = Service.GameInteropProvider.HookFromAddress<PacketDispatcher.Delegates.HandleQueueUpdatePacket>((nint)PacketDispatcher.MemberFunctionPointers.HandleQueueUpdatePacket, ProcessContentsFinderUpdatePacket2Detour);
        queueInfoWithdrawQueueHook = Service.GameInteropProvider.HookFromAddress<ContentsFinderQueueInfo.Delegates.OnQueueWithdrawn>((nint)ContentsFinderQueueInfo.MemberFunctionPointers.OnQueueWithdrawn, QueueInfoWithdrawQueueDetour);
        queueInfoDutyPopHook = Service.GameInteropProvider.HookFromAddress<ContentsFinderQueueInfo.Delegates.OnQueuePop>((nint)ContentsFinderQueueInfo.MemberFunctionPointers.OnQueuePop, QueueInfoDutyPopDetour);

        queueInfoProcessInfoStateHook.Enable();
        processContentsFinderUpdatePacket2Hook.Enable();
        queueInfoWithdrawQueueHook.Enable();
        queueInfoDutyPopHook.Enable();

        Service.ClientState.ZoneInit += ZoneInit;
    }

    private void QueueInfoProcessInfoStateDetour(ContentsFinderQueueInfo* @this, ContentsFinderQueueState newState, QueueInfoState* newInfoState)
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

        try
        {
            OnUpdateQueue?.Invoke(BaseQueueUpdate.Create(*newInfoState));
        }
        catch (Exception e)
        {
            Log.ErrorNotify(e, "Error invoking QueueInfoProcessInfoStateDetour", "Please report this as a bug");
        }

        queueInfoProcessInfoStateHook.Original(@this, newState, newInfoState);
    }

    private void ProcessContentsFinderUpdatePacket2Detour(QueueUpdatePacket* packet)
    {
        //Log.Debug("Packet2");
        //Log.Debug($"State: {packet->QueueState}");
        //Log.Debug($"Lang: {packet->LanguageFlags}");
        //Log.Debug($"Began: {packet->BeganQueue}");
        //Log.Debug($"Job: {packet->ClassJobId}");
        //Log.Debug($"Flags: {packet->Flags}");
        //Log.Debug($"Roulette: {packet->RouletteId}");
        //Log.Debug($"Conditions: {string.Join(", ", new Span<int>(packet->ContentFinderConditions, 5).ToArray())}");

        try
        {
            if (packet->BeganQueue)
            {
                var makeup = PartyMakeup.TryCreate();
                var queueInfo = new QueueInfo(packet);
                var languages = packet->LanguageFlags;
                OnEnterQueue?.Invoke(queueInfo, (QueueLanguage)languages, LuminaSheets.CreateRowRef<ClassJob>(packet->ClassJobId), makeup);
            }
        }
        catch (Exception e)
        {
            Log.ErrorNotify(e, "Error invoking ProcessContentsFinderUpdatePacket2Detour", "Please report this as a bug");
        }
        processContentsFinderUpdatePacket2Hook.Original(packet);
    }

    private void QueueInfoWithdrawQueueDetour(ContentsFinderQueueInfo* @this, uint logMessageId, ulong contentId)
    {
        //Log.Debug("Withdraw");
        //Log.Debug($"Log: {LuminaSheets.LogMessage.GetRow(logMessageId).Text.ExtractText()} ({logMessageId})");
        //Log.Debug($"Content: {contentId:X16}");

        try
        {
            OnWithdrawQueue?.Invoke(LuminaSheets.CreateRowRef<LogMessage>(logMessageId));
        }
        catch (Exception e)
        {
            Log.ErrorNotify(e, "Error invoking QueueInfoWithdrawQueueDetour", "Please report this as a bug");
        }
        queueInfoWithdrawQueueHook.Original(@this, logMessageId, contentId);
    }

    private void QueueInfoDutyPopDetour(ContentsFinderQueueInfo* @this, ContentsFinderQueueState newState, uint contentId, nint a4, bool isInProgressParty, ContentsFinder.LootRule lootRule, ulong inProgressPartyStartTimestamp, nint a8, bool isUnrestrictedParty, bool isMinimalIL, bool isSilenceEcho, bool isExplorerMode, bool isLevelSync, bool isLimitedLeveling)
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

        try
        {
            var rouletteId = ContentsFinder.Instance()->QueueInfo.QueuedContentRouletteId;
            var queueInfo = new QueueInfo()
            {
                Content = rouletteId != 0
                    ? [(RowRef)LuminaSheets.CreateRowRef<Lumina.Excel.Sheets.ContentRoulette>(rouletteId)]
                    : [(RowRef)LuminaSheets.CreateRowRef<ContentFinderCondition>(contentId)],
                Flags = new()
                {
                    LootRule = (LootRuleFlags)lootRule,
                    IsUnrestrictedParty = isUnrestrictedParty,
                    IsMinIlvl = isMinimalIL,
                    IsSilenceEcho = isSilenceEcho,
                    IsExplorer = isExplorerMode,
                    IsLevelSynced = isLevelSync,
                    IsLimitedLeveling = isLimitedLeveling,
                    InProgressParty = isInProgressParty
                }
            };
            DateTime? inProgressStartTimestamp = inProgressPartyStartTimestamp == 0 ? null : DateTimeOffset.FromUnixTimeSeconds((long)inProgressPartyStartTimestamp).UtcDateTime;
            OnPopQueue?.Invoke(queueInfo, inProgressStartTimestamp);
        }
        catch (Exception e)
        {
            Log.ErrorNotify(e, "Error invoking QueueInfoDutyPopDetour", "Please report this as a bug");
        }

        queueInfoDutyPopHook.Original(@this, newState, contentId, a4, isInProgressParty, lootRule, inProgressPartyStartTimestamp, a8, isUnrestrictedParty, isMinimalIL, isSilenceEcho, isExplorerMode, isLevelSync, isLimitedLeveling);
    }

    private void ZoneInit(ZoneInitEventArgs args)
    {
        // Log.Debug("Zone Init");
        // Log.Debug($"Territory: {args.TerritoryType.RowId}");
        // Log.Debug($"Instance: {args.Instance}");
        // Log.Debug($"Condition: {args.ContentFinderCondition.RowId}");
        // Log.Debug($"Weather: {args.Weather.RowId}");
        // Log.Debug($"Festivals: {string.Join(", ", args.ActiveFestivals.Select(f => $"{f.Festival.RowId} - {f.FestivalPhase}"))}");

        try
        {
            if (args.ContentFinderCondition.RowId != 0)
                OnEnterContent?.Invoke(args.ContentFinderCondition);
        }
        catch (Exception e)
        {
            Log.ErrorNotify(e, "Error invoking ZoneInit event", "Please report this as a bug");
        }
    }

    public void Dispose()
    {
        queueInfoProcessInfoStateHook?.Dispose();
        processContentsFinderUpdatePacket2Hook?.Dispose();
        queueInfoWithdrawQueueHook?.Dispose();
        queueInfoDutyPopHook?.Dispose();
        Service.ClientState.ZoneInit -= ZoneInit;
    }
}

[Flags]
public enum QueueLanguage
{
    Japanese = 1,
    English = 2,
    German = 4,
    French = 8
}