using System;

namespace Waitingway.Api.Login.Models;

public sealed record DeleteNotificationData
{
    public required bool Successful { get; init; }
    public required uint QueueStartSize { get; init; }
    public required uint QueueEndSize { get; init; }
    public required uint Duration { get; init; }
    public required string? ErrorMessage { get; init; }
    public required int? ErrorCode { get; init; }
    public required DateTime? IdentifyTimeout { get; init; }
}
