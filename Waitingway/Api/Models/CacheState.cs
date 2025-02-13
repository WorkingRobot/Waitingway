namespace Waitingway.Api.Models;

public enum CacheState : byte
{
    Found,
    InProgress,
    NotFound,
    Failed
}
