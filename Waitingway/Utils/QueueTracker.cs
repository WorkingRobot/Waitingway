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
        }

        [JsonIgnore]
        public string CharacterName { get; }
        [JsonIgnore]
        public ushort HomeWorldId { get; }

        public ushort WorldId { get; }
        public bool Successful { get; private set; }
        public DateTime StartTime { get; }
        public DateTime EndTime { get; private set; }

        private List<Position> _positions { get; }
        public IReadOnlyList<Position> Positions => _positions;

        [JsonIgnore]
        public DateTime EstimatedEndTime => EstimateEndTime(DateTime.UtcNow);

        [JsonIgnore]
        public Position? CurrentPosition => Positions.Count == 0 ? null : Positions[^1];

        public Recap(string characterName, ushort homeWorldId, ushort worldId, DateTime startTime)
        {
            CharacterName = characterName;
            HomeWorldId = homeWorldId;
            WorldId = worldId;
            StartTime = startTime;
            _positions = [];
        }

        public void AddPosition(int positionNumber)
        {
            _positions.Add(new Position { PositionNumber = positionNumber, Time = DateTime.UtcNow });
        }

        public void Complete(bool successful)
        {
            EndTime = DateTime.UtcNow;
            Successful = successful;
        }

        public DateTime EstimateEndTime(DateTime now)
        {
            var config = Service.Configuration;
            return EstimateEndTime(now, config.DefaultRate, config.Estimator switch
            {
                EstimatorType.Geometric => Estimator.GeometricWeight,
                EstimatorType.MinorGeometric => Estimator.MinorGeometricWeight,
                EstimatorType.Inverse => Estimator.InverseWeight,
                EstimatorType.ShiftedInverse => Estimator.ShiftedInverseWeight,
                _ => throw new NotSupportedException()
            });
        }

        private DateTime EstimateEndTime(DateTime now, float defaultPositionsPerMinute, Func<int, double> weightFunction)
        {
            var history = Positions.Select(p => (p.Time, p.PositionNumber));
            return Estimator.EstimateRate(history, now, defaultPositionsPerMinute, weightFunction);
        }
    }

    public bool InQueue => CurrentRecap != null;

    public event Action<Recap>? OnBeginQueue;

    public event Action<Recap>? OnUpdateQueue;

    public event Action<Recap>? OnCompleteQueue;

    public Recap? CurrentRecap { get; private set; }

    public QueueTracker()
    {
        Service.Hooks.OnEnterQueue += OnEnterQueue;
        Service.Hooks.OnExitQueue += OnExitQueue;
        Service.Hooks.OnNewQueuePosition += OnNewQueuePosition;
    }

    private void OnEnterQueue(string characterName, ushort homeWorldId, ushort worldId)
    {
        CurrentRecap = new(characterName, homeWorldId, worldId, DateTime.UtcNow);
        OnBeginQueue?.Invoke(CurrentRecap);
    }

    private void OnExitQueue(bool isSuccessful)
    {
        if (CurrentRecap is { } recap)
        {
            recap.Complete(isSuccessful);
            OnCompleteQueue?.Invoke(recap);
            CurrentRecap = null;
        }
    }

    private void OnNewQueuePosition(int newPosition)
    {
        if (CurrentRecap is { } recap)
        {
            recap.AddPosition(newPosition);
            OnUpdateQueue?.Invoke(recap);
        }
        else
            Log.ErrorNotify($"Received new queue position ({newPosition}) while prior knowlege of queue. Did you install/enable Waitingway while queued?", "Unexpected Queue Update");
    }

    public void Dispose()
    {
        Service.Hooks.OnEnterQueue -= OnEnterQueue;
        Service.Hooks.OnExitQueue -= OnExitQueue;
        Service.Hooks.OnNewQueuePosition -= OnNewQueuePosition;
    }
}
