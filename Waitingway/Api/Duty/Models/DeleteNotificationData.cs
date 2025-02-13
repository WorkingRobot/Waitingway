namespace Waitingway.Api.Duty.Models;

public sealed record DeleteNotificationData
{
    public required byte? PositionStart { get; init; }
    public required byte? PositionEnd { get; init; }
    public required uint Duration { get; init; }
    public required ushort? ResultingContent { get; init; }
    public required string? ErrorMessage { get; init; }
    public required ushort? ErrorCode { get; init; }
}
