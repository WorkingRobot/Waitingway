using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Serialization;
using Lumina.Excel;
using Lumina.Excel.Sheets;
using Waitingway.Api.Duty.Models;
using Waitingway.Utils;
using static Waitingway.Hooks.DutyQueue;
using Action = System.Action;

namespace Waitingway.Api.Duty;

public sealed class DutyQueueTracker : IDisposable
{
    public sealed record Recap
    {
        public sealed record PopInfo
        {
            public required DateTime Timestamp { get; init; }
            public required ContentFlags ResultingFlags { get; init; }
            public ushort? ResultingContent { get; set; }
            public DateTime? InProgressBeginTimestamp { get; init; }
        }

        [JsonIgnore]
        public string CharacterName { get; }

        public byte? QueuedRoulette { get; }
        public ushort[]? QueuedContent { get; }
        public byte QueuedJob { get; }
        public ContentFlags QueuedFlags { get; }
        public QueueLanguage QueuedLanguages { get; }

        public ushort WorldId { get; }
        public PartyMakeup? Party { get; }

        public DateTime StartTime { get; }
        public DateTime? EndTime { get; private set; }

        public ushort? WithdrawMessage { get; private set; }

        private List<BaseQueueUpdate> _updates { get; }
        public IReadOnlyList<BaseQueueUpdate> Updates => _updates;

        private List<PopInfo> _pops { get; }
        public IReadOnlyList<PopInfo> Pops => _pops;

        [JsonIgnore]
        public RouletteRole? Role =>
            LuminaSheets.ClassJob.GetRowOrDefault(QueuedJob)?.Role switch
            {
                1 => RouletteRole.Tank,
                2 or 3 => RouletteRole.DPS,
                4 => RouletteRole.Healer,
                _ => null
            };

        [JsonIgnore]
        public BaseQueueUpdate? LastUpdate => _updates.LastOrDefault();

        [JsonIgnore]
        public PopInfo? LastPop => _pops.LastOrDefault();

        [JsonIgnore]
        public bool Successful => !WithdrawMessage.HasValue && EndTime.HasValue;

        public Recap(string characterName, ushort worldId, byte classJob, QueueLanguage languages, QueueInfo queueInfo, PartyMakeup? party, DateTime startTime)
        {
            if (queueInfo.Content?.Length is null or 0)
                throw new ArgumentException("No content was queued for.", nameof(queueInfo));

            CharacterName = characterName;
            WorldId = worldId;
            QueuedJob = classJob;
            QueuedLanguages = languages;
            Party = party;
            StartTime = startTime;
            QueuedFlags = queueInfo.Flags;
            QueuedRoulette = (byte?)queueInfo.Content[0].GetValueOrDefault<ContentRoulette>()?.RowId;
            QueuedContent = QueuedRoulette.HasValue
                ? null
                : queueInfo.Content?
                    .Select(x => (ushort?)x.GetValueOrDefault<ContentFinderCondition>()?.RowId)
                    .Where(c => c.HasValue)
                    .Select(c => c!.Value)
                    .ToArray();
            _updates = [];
            _pops = [];
        }

        public void AddUpdate(BaseQueueUpdate update)
        {
            _updates.Add(update);
        }

        public void PopQueue(DateTime endTime, QueueInfo queueInfo, DateTime? resultingInProgressTimestamp)
        {
            _pops.Add(new()
            {
                Timestamp = endTime,
                ResultingFlags = queueInfo.Flags,
                ResultingContent = (ushort?)queueInfo.Content[0].GetValueOrDefault<ContentFinderCondition>()?.RowId,
                InProgressBeginTimestamp = resultingInProgressTimestamp
            });
        }

        public void WithdrawQueue(DateTime endTime, ushort messageId)
        {
            EndTime = endTime;
            WithdrawMessage = messageId;
        }

        public void CompleteQueue(DateTime endTime, ushort resultingContent)
        {
            EndTime = endTime;
            _pops[^1].ResultingContent = resultingContent;
        }
    }

    public enum QueueState
    {
        NotQueued,
        Queued,
        Popped
    }

    public QueueState CurrentState { get; private set; }

    // New recap
    public event Action? OnBeginQueue;

    // New update
    public event Action? OnUpdateQueue;

    // Queue popped (will not be run on withdraw); Recap can still be updated upon successful queue into a roulette
    public event Action? OnPopQueue;

    // Completed recap; Recap will no longer be updated
    public event Action? OnFinalizeQueue;

    public Recap? CurrentRecap { get; private set; }

    public DutyQueueTracker()
    {
        Service.Hooks.Duty.OnEnterQueue += CbOnEnterQueue;
        Service.Hooks.Duty.OnWithdrawQueue += CbOnWithdrawQueue;
        Service.Hooks.Duty.OnPopQueue += CbOnPopQueue;
        Service.Hooks.Duty.OnUpdateQueue += CbOnUpdateQueue;
        Service.Hooks.Duty.OnEnterContent += CbOnEnterContent;
        CurrentState = QueueState.NotQueued;
    }

    private void CbOnEnterQueue(QueueInfo queueInfo, QueueLanguage languages, RowRef<ClassJob> classJob, PartyMakeup? party)
    {
        CurrentRecap = new(Service.Objects.LocalPlayer!.Name.TextValue, (ushort)Service.Objects.LocalPlayer!.CurrentWorld.RowId, (byte)classJob.RowId, languages, queueInfo, party, DateTime.UtcNow);
        CurrentState = QueueState.Queued;
        OnBeginQueue?.Invoke();
    }

    private void CbOnWithdrawQueue(RowRef<LogMessage> message)
    {
        if (CurrentRecap is not { } recap)
            return;

        CurrentState = QueueState.NotQueued;
        recap.WithdrawQueue(DateTime.UtcNow, (ushort)message.RowId);
        OnFinalizeQueue?.Invoke();
        CurrentRecap = null;
    }

    private void CbOnPopQueue(QueueInfo queueInfo, DateTime? inProgressStartTime)
    {
        if (CurrentRecap is not { } recap)
        {
            Log.Warn($"Queue popped without prior knowlege of queue. Did you install/enable Waitingway while queued?");
            return;
        }

        CurrentState = QueueState.Popped;
        recap.PopQueue(DateTime.UtcNow, queueInfo, inProgressStartTime);
        OnPopQueue?.Invoke();
    }

    private void CbOnUpdateQueue(BaseQueueUpdate update)
    {
        if (CurrentRecap is not { } recap)
        {
            Log.Warn($"Received new queue update without prior knowlege of queue. Did you install/enable Waitingway while queued?");
            return;
        }

        if (CurrentState == QueueState.NotQueued)
            return;

        // If a queued up party member withdraws, the server goes back to a
        // queue update packet; withdraw packets or update2 packets are not used.
        if (CurrentState == QueueState.Popped)
            CurrentState = QueueState.Queued;

        recap.AddUpdate(update);
        OnUpdateQueue?.Invoke();
    }

    private void CbOnEnterContent(RowRef<ContentFinderCondition> content)
    {
        if (CurrentRecap is not { } recap)
            return;

        if (CurrentState != QueueState.Popped)
            return;

        CurrentState = QueueState.NotQueued;
        recap.CompleteQueue(DateTime.UtcNow, (ushort)content.RowId);
        OnFinalizeQueue?.Invoke();
        CurrentRecap = null;
    }

    public void Dispose()
    {
        Service.Hooks.Duty.OnEnterQueue -= CbOnEnterQueue;
        Service.Hooks.Duty.OnWithdrawQueue -= CbOnWithdrawQueue;
        Service.Hooks.Duty.OnPopQueue -= CbOnPopQueue;
        Service.Hooks.Duty.OnUpdateQueue -= CbOnUpdateQueue;
        Service.Hooks.Duty.OnEnterContent -= CbOnEnterContent;
    }
}
