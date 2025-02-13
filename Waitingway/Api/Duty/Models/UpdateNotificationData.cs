using System;
using System.Text.Json.Serialization;
using static Waitingway.Hooks.DutyQueue;

namespace Waitingway.Api.Duty.Models;

[JsonDerivedType(typeof(QueueUpdateNotificationData), typeDiscriminator: 0)]
[JsonDerivedType(typeof(PopUpdateNotificationData), typeDiscriminator: 1)]
public record UpdateNotificationData
{

}

public sealed record QueueUpdateNotificationData : UpdateNotificationData
{
    public required DateTime? EstimatedTime { get; init; }
    public required BaseQueueUpdate Update { get; init; }
}

public sealed record PopUpdateNotificationData : UpdateNotificationData
{
    public required DateTime Timestamp { get; init; }
    public required ushort? ResultingContent { get; init; }
    public required DateTime? InProgressBeginTimestamp { get; init; }
}
