using static Waitingway.Hooks.DutyQueue;

namespace Waitingway.Api.Duty.Models;

public sealed record RouletteSize
{
    public required ushort WorldId { get; init; }
    public required QueueLanguage Languages { get; init; }
    public required byte RouletteId { get; init; }
    public required RouletteRole Role { get; init; }

    public required byte? Size { get; init; }
    public required byte? EstimatedWaitTime { get; init; }
}
