using System.Collections.Frozen;
using System.Collections.Generic;

namespace Waitingway.Utils;

public sealed record World
{
    private static FrozenDictionary<ushort, World> Worlds { get; }

    public required ushort WorldId { get; init; }
    public required string WorldName { get; init; }

    public required ushort DatacenterId { get; init; }
    public required string DatacenterName { get; init; }

    public required ushort RegionId { get; init; }
    public required string RegionName { get; init; }

    public required bool IsCloud { get; init; }

    private enum RegionType : byte
    {
        Internal,
        Japan,
        NorthAmerica,
        Europe,
        Oceania,
        China,
        Korea,
        Cloud
    }

    static World()
    {
        var worlds = new Dictionary<ushort, World>();

        foreach (var world in LuminaSheets.World)
        {
            //if (!world.IsPublic)
            //    continue;

            if (world.DataCenter.ValueNullable is not { } datacenter)
                continue;

            var region = (RegionType)datacenter.Region;

            if (region == RegionType.Internal)
                continue;

            var regionName = region switch
            {
                RegionType.Japan => "JP",
                RegionType.NorthAmerica => "NA",
                RegionType.Europe => "EU",
                RegionType.Oceania => "OC",
                RegionType.China => "CN",
                RegionType.Korea => "KR",
                RegionType.Cloud => "Cloud",
                _ => "Unknown"
            };

            worlds.Add((ushort)world.RowId, new World
            {
                WorldId = (ushort)world.RowId,
                WorldName = world.Name.ExtractText(),
                DatacenterId = (ushort)datacenter.RowId,
                DatacenterName = datacenter.Name.ExtractText(),
                RegionId = (ushort)region,
                RegionName = regionName,
                IsCloud = datacenter.Unknown0
            });
        }

        Worlds = worlds.ToFrozenDictionary();
    }

    private World() { }

    public static World? GetWorld(ushort worldId) =>
        Worlds.TryGetValue(worldId, out var world) ? world : null;

    public static IEnumerable<World> GetWorlds() =>
        Worlds.Values;
}
