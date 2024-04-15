using Dalamud.Configuration;
using System;

namespace Waitingway;

public enum EstimatorType
{
    Geometric,
    MinorGeometric,
    Inverse,
    ShiftedInverse,
}

[Serializable]
public class Configuration : IPluginConfiguration
{
    public int Version { get; set; } = 2;

    public string RemoteServer { get; set; } = "https://waiting.camora.dev";

    public string ClientId { get; init; } = Guid.NewGuid().ToString();
    public string ClientSalt { get; init; } = Guid.NewGuid().ToString().Split('-')[0];

    public EstimatorType Estimator { get; set; } = EstimatorType.Geometric;
    public float DefaultRate { get; set; } = 75;
    public int MinimumPositionThreshold { get; set; } = 40;

    public void Save() =>
        Service.PluginInterface.SavePluginConfig(this);
}
