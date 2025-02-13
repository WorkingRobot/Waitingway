namespace Waitingway.Api.Login.Models;

public sealed record QueueSize
{
    public required ushort WorldId { get; init; }
    public required int Size { get; init; }
}
