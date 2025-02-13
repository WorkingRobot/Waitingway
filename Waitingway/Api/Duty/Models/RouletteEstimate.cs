using System;

namespace Waitingway.Api.Duty.Models;

public sealed record RouletteEstimate
{
    public required ushort DatacenterId { get; init; }
    public required byte RouletteId { get; init; }
    public required RouletteRole Role { get; init; }

    public required DateTime LastUpdate { get; init; }
    public required TimeSpan WaitTime { get; init; }
    public required byte Size { get; init; }
    public required byte EstimatedWaitTime { get; init; }
}
