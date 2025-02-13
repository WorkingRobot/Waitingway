using System;

namespace Waitingway.Api.Login.Models;

public sealed record CreateNotificationData
{
    public required string CharacterName { get; init; }
    public required ushort HomeWorldId { get; init; }
    public required ushort WorldId { get; init; }
    public required uint Position { get; init; }
    public required DateTime UpdatedAt { get; init; }
    public required DateTime EstimatedTime { get; init; }
}
