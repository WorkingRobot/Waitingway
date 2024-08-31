using Newton = Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Serialization;

namespace Waitingway.Utils;

public sealed class QueueTracker : IDisposable
{
    public sealed record Recap
    {
        public sealed record Position
        {
            [JsonPropertyName("position")]
            public required int PositionNumber { get; init; }
            public required DateTime Time { get; init; }
            public required DateTime? IdentifyTime { get; init; }
        }

        public sealed record ErrorInfo
        {
            public required int Type { get; init; }
            public required int Code { get; init; }
            public required string Info { get; init; }
            public required ushort ErrorRow { get; init; }
        }

        [JsonIgnore]
        public string CharacterName { get; }
        [JsonIgnore]
        public ulong CharacterContentId { get; }
        [JsonIgnore]
        public ushort HomeWorldId { get; }

        public ushort WorldId { get; }
        public bool FreeTrial { get; }
        public bool Successful { get; private set; }
        public bool Reentered { get; private set; }
        public ErrorInfo? Error { get; private set; }
        public DateTime StartTime { get; }
        public DateTime EndTime { get; private set; }
        public DateTime? EndIdentifyTime { get; private set; }

        private List<Position> _positions { get; }
        public IReadOnlyList<Position> Positions => _positions;

        [JsonIgnore]
        [Newton.JsonIgnore]
        public DateTime EstimatedEndTime => EstimateEndTime(DateTime.UtcNow);

        [JsonIgnore]
        [Newton.JsonIgnore]
        public Position? CurrentPosition => Positions.Count == 0 ? null : Positions[^1];

        [JsonIgnore]
        [Newton.JsonIgnore]
        public DateTime? LastIdentifyTime => EndIdentifyTime ?? CurrentPosition?.IdentifyTime;

        [JsonIgnore]
        [Newton.JsonIgnore]
        public DateTime? IdentifyTimeout => LastIdentifyTime?.AddSeconds(220);

        [JsonIgnore]
        [Newton.JsonIgnore]
        public DateTime? NextPlannedIdentifyTime => NextIdentifyTime is { } nextTime ? DateTime.UtcNow + nextTime : LastIdentifyTime?.AddSeconds(30);

        [JsonIgnore]
        [Newton.JsonIgnore]
        public bool IsIdentifyExpired => !LastIdentifyTime.HasValue || DateTime.UtcNow >= IdentifyTimeout;

        public Recap(string characterName, ulong characterContentId, bool freeTrial, ushort homeWorldId, ushort worldId, DateTime startTime)
        {
            CharacterName = characterName;
            CharacterContentId = characterContentId;
            FreeTrial = freeTrial;
            HomeWorldId = homeWorldId;
            WorldId = worldId;
            StartTime = startTime;
            _positions = [];
            Reentered = false;
        }

        public void AddPosition(Position position)
        {
            _positions.Add(position);
        }

        public void MarkComplete(DateTime endTime, DateTime? endIdentifyTime)
        {
            Successful = true;
            EndTime = endTime;
            EndIdentifyTime = endIdentifyTime;
        }

        public void MarkCancelled(DateTime endTime, DateTime? endIdentifyTime)
        {
            Successful = false;
            EndTime = endTime;
            EndIdentifyTime = endIdentifyTime;
        }

        public void MarkFailed(ErrorInfo error, DateTime endTime, DateTime? endIdentifyTime)
        {
            Successful = false;
            Error = error;
            EndTime = endTime;
            EndIdentifyTime = endIdentifyTime;
        }

        public void ReEnterQueue()
        {
            if (Successful)
                throw new InvalidOperationException("Cannot re-enter a successful queue.");
            if (EndTime == default)
                throw new InvalidOperationException("Cannot re-enter a queue that has not ended.");

            Successful = false;
            Reentered = true;
            Error = null;
            EndTime = default;
            EndIdentifyTime = null;
        }

        public DateTime EstimateEndTime(DateTime now)
        {
            var config = Service.Configuration;
            var endTime = EstimateEndTime(now, config.DefaultRate, config.Estimator switch
            {
                EstimatorType.Geometric => Estimator.GeometricWeight,
                EstimatorType.MinorGeometric => Estimator.MinorGeometricWeight,
                EstimatorType.Inverse => Estimator.InverseWeight,
                EstimatorType.ShiftedInverse => Estimator.ShiftedInverseWeight,
                _ => throw new NotSupportedException()
            });
            return Estimator.RoundEstimate(endTime, NextPlannedIdentifyTime, TimeSpan.FromSeconds(config.IdentifyLatency), TimeSpan.FromSeconds(config.LoginLatency));
        }

        private DateTime EstimateEndTime(DateTime now, float defaultPositionsPerMinute, Func<int, double> weightFunction)
        {
            var history = Positions.Select(p => (p.Time, p.PositionNumber));
            return Estimator.EstimateRate(history, now, defaultPositionsPerMinute, weightFunction);
        }
    }

    public enum QueueState
    {
        NotQueued,
        Entered,
        SentIdentify,
        WaitingForNextIdentify
    }

    public QueueState CurrentState { get; private set; }

    // New recap
    public event Action? OnBeginQueue;

    // New position
    public event Action? OnUpdateQueue;

    // Recap ended
    public event Action? OnCompleteQueue;

    public Recap? CurrentRecap { get; private set; }

    public static TimeSpan? NextIdentifyTime =>
        Hooks.AgentLobbyGetTimeSinceLastIdentify() is { } lastTime ?
            TimeSpan.FromSeconds(30) - TimeSpan.FromMilliseconds(lastTime) :
            null;

    private DateTime? LastIdentifyTime { get; set; }

    public QueueTracker()
    {
        Service.Hooks.OnEnterQueue += OnEnterQueue;
        Service.Hooks.OnCancelQueue += OnCancelQueue;
        Service.Hooks.OnFailedQueue += OnFailedQueue;
        Service.Hooks.OnExitQueue += OnExitQueue;
        Service.Hooks.OnSendIdentify += OnSendIdentify;
        Service.Hooks.OnNewQueuePosition += OnNewQueuePosition;
        CurrentState = QueueState.NotQueued;
    }

    private void OnEnterQueue(string characterName, ulong characterContentId, bool isFreeTrial, ushort homeWorldId, ushort worldId)
    {
        if (Service.Configuration.TakeFailedRecap(characterContentId) is { } failedRecap &&
            !failedRecap.IsIdentifyExpired &&
            failedRecap.WorldId == worldId)
        {
            failedRecap.ReEnterQueue();
            CurrentRecap = failedRecap;
        }
        else
            CurrentRecap = new(characterName, characterContentId, isFreeTrial, homeWorldId, worldId, DateTime.UtcNow);
        CurrentState = QueueState.Entered;
        OnBeginQueue?.Invoke();
    }

    private void OnCancelQueue()
    {
        if (CurrentRecap is not { } recap)
        {
            Log.ErrorNotify($"Cancelled queue without prior knowlege of queue. Did you install/enable Waitingway while queued?", "Unexpected Queue Update");
            return;
        }

        CurrentState = QueueState.NotQueued;
        recap.MarkCancelled(DateTime.UtcNow, LastIdentifyTime);
        LastIdentifyTime = null;
        OnCompleteQueue?.Invoke();
        CurrentRecap = null;
        Service.Configuration.AddFailedRecap(recap);
    }

    private void OnFailedQueue(int type, int code, string info, ushort errorRow)
    {
        if (CurrentRecap is not { } recap)
            return;

        CurrentState = QueueState.NotQueued;
        recap.MarkFailed(new()
        {
            Type = type,
            Code = code,
            Info = info,
            ErrorRow = errorRow
        }, DateTime.UtcNow, LastIdentifyTime);
        LastIdentifyTime = null;
        OnCompleteQueue?.Invoke();
        CurrentRecap = null;
        Service.Configuration.AddFailedRecap(recap);
    }

    private void OnExitQueue()
    {
        if (CurrentRecap is not { } recap)
        {
            LastIdentifyTime = null;
            Log.ErrorNotify($"Exited queue without prior knowlege of queue. Did you install/enable Waitingway while queued?", "Unexpected Queue Update");
            return;
        }

        CurrentState = QueueState.NotQueued;
        recap.MarkComplete(DateTime.UtcNow, LastIdentifyTime);
        LastIdentifyTime = null;
        OnCompleteQueue?.Invoke();
        CurrentRecap = null;
    }

    private void OnSendIdentify()
    {
        CurrentState = QueueState.SentIdentify;
        LastIdentifyTime = DateTime.UtcNow;
    }

    private void OnNewQueuePosition(int newPosition)
    {
        if (CurrentRecap is not { } recap)
        {
            LastIdentifyTime = null;
            Log.ErrorNotify($"Received new queue position ({newPosition}) without prior knowlege of queue. Did you install/enable Waitingway while queued?", "Unexpected Queue Update");
            return;
        }

        CurrentState = QueueState.WaitingForNextIdentify;
        if (newPosition > 0)
        {
            recap.AddPosition(new() { PositionNumber = newPosition, Time = DateTime.UtcNow, IdentifyTime = LastIdentifyTime });
            LastIdentifyTime = null;
            OnUpdateQueue?.Invoke();
        }
        else
            LastIdentifyTime = null;
    }

    public void Dispose()
    {
        Service.Hooks.OnEnterQueue -= OnEnterQueue;
        Service.Hooks.OnExitQueue -= OnExitQueue;
        Service.Hooks.OnNewQueuePosition -= OnNewQueuePosition;
    }
}
