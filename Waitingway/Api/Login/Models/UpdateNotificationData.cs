using System;

namespace Waitingway.Api.Login.Models;

public sealed record UpdateNotificationData
{
    public required uint Position { get; init; }
    public required DateTime UpdatedAt { get; init; }
    public required DateTime EstimatedTime { get; init; }
}
