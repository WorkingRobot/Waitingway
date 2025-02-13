using System;

namespace Waitingway.Api.Login.Models;

public sealed record QueueEstimate
{
    public required ushort WorldId { get; init; }

    public required DateTime LastUpdate { get; init; }
    public required uint LastSize { get; init; }
    public required TimeSpan LastDuration { get; init; }

    // public required TimeSpan EstimatedQueueDuration { get; init; }
}
