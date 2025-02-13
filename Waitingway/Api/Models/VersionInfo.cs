using System;

namespace Waitingway.Api.Models;

public sealed record VersionInfo
{
    public required string Name { get; init; }
    public required string Authors { get; init; }
    public required string Description { get; init; }
    public required string Repository { get; init; }
    public required string Profile { get; init; }
    public required string Version { get; init; }
    public required uint VersionMajor { get; init; }
    public required uint VersionMinor { get; init; }
    public required uint VersionPatch { get; init; }
    public required string SupportedVersion { get; init; }
    public required uint SupportedVersionMajor { get; init; }
    public required uint SupportedVersionMinor { get; init; }
    public required uint SupportedVersionPatch { get; init; }
    public required DateTime BuildTime { get; init; }
}
