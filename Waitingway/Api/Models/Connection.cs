using System;

namespace Waitingway.Api.Models;

public sealed record Connection
{
    public required DateTime CreatedAt { get; init; }

    public required ulong ConnUserId { get; init; }
    public required string Username { get; init; }
    public required string DisplayName { get; init; }
}
