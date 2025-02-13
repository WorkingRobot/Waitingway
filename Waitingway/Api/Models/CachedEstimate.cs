namespace Waitingway.Api.Models;

public readonly record struct CachedEstimate<T> where T : class
{
    public required CacheState State { get; init; }

    public required T? Estimate { get; init; }
}
