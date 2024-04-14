using System;
using System.Collections.Generic;
using System.Linq;

namespace Waitingway.Utils;

public sealed class QueueTracker : IDisposable
{
    public sealed record Recap
    {
        public sealed record Position
        {
            public int PositionNumber { get; init; }
            public DateTime Time { get; init; }
        }

        public DateTime StartTime { get; }
        public List<Position> Positions { get; }
        public DateTime EndTime { get; set; }
        public bool WasSuccessful { get; set; }

        public Recap(DateTime startTime)
        {
            StartTime = startTime;
            Positions = new();
        }
    }

    public bool InQueue => CurrentRecap != null;
    public int? Position => CurrentRecap?.Positions.LastOrDefault()?.PositionNumber;

    public event Action<int>? OnPositionUpdate;

    public event Action<Recap>? OnRecap;

    private Recap? CurrentRecap { get; set; }

    public QueueTracker()
    {
        Service.Hooks.OnEnterQueue += OnEnterQueue;
        Service.Hooks.OnExitQueue += OnExitQueue;
        Service.Hooks.OnNewQueuePosition += OnNewQueuePosition;
    }

    private void OnEnterQueue()
    {
        Log.Debug("Entered queue");
        CurrentRecap = new(DateTime.UtcNow);
    }

    private void OnExitQueue(bool isSuccessful)
    {
        Log.Debug($"Exited queue (successful login: {isSuccessful})");
        if (CurrentRecap is { } recap)
        {
            recap.EndTime = DateTime.UtcNow;
            recap.WasSuccessful = isSuccessful;
            OnRecap?.Invoke(recap);
            CurrentRecap = null;
        }
    }

    private void OnNewQueuePosition(int newPosition)
    {
        Log.Debug($"New queue position: {newPosition}");
        if (CurrentRecap is { } recap)
            recap.Positions.Add(new Recap.Position { PositionNumber = newPosition, Time = DateTime.UtcNow });
        else
            Log.Error($"Received new queue position ({newPosition}) while not in queue");
        OnPositionUpdate?.Invoke(newPosition);
    }

    public void Dispose()
    {
        Service.Hooks.OnEnterQueue -= OnEnterQueue;
        Service.Hooks.OnExitQueue -= OnExitQueue;
        Service.Hooks.OnNewQueuePosition -= OnNewQueuePosition;
    }
}
