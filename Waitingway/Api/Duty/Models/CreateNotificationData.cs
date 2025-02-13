using System;
using static Waitingway.Hooks.DutyQueue;

namespace Waitingway.Api.Duty.Models;

public sealed record CreateNotificationData
{
    public required string CharacterName { get; init; }
    public required ushort HomeWorldId { get; init; }
    public required byte QueuedJob { get; init; }
    public required byte? QueuedRoulette { get; init; }
    public required ushort[]? QueuedContent { get; init; }
    public required DateTime? EstimatedTime { get; init; }
    public required BaseQueueUpdate Update { get; init; }
}
