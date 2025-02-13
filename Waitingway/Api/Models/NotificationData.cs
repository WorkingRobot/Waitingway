namespace Waitingway.Api.Models;

public sealed record NotificationData
{
    public required string Nonce { get; init; }
    public required string Data { get; init; }
}
