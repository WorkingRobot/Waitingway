using Dalamud.Configuration;
using Newtonsoft.Json;
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

    private static Uri DefaultServerUri => new("https://waiting.camora.dev/");

    [JsonProperty(nameof(ServerUri))]
    private string ServerUriInternal { get; set; } = DefaultServerUri.AbsoluteUri;

    [JsonIgnore]
    public Uri ServerUri
    {
        get
        {
            if (!Uri.TryCreate(ServerUriInternal, UriKind.Absolute, out var ret))
                return DefaultServerUri;
            return ret;
        }
        set
        {
            ServerUriInternal = value.AbsoluteUri;
            Service.Api.RefreshHttpConfiguration();
        }
    }

    [JsonProperty("ClientId_DoNotShareThisWithAnyone_TreatItLikeAPassword")]
    public string ClientId { get; init; } = Guid.NewGuid().ToString("N").ToUpperInvariant();

    public EstimatorType Estimator { get; set; } = EstimatorType.Geometric;
    public float DefaultRate { get; set; } = 100;
    public int MinimumPositionThreshold { get; set; } = 40;
    public int NotificationThreshold { get; set; }

    public void Save() =>
        Service.PluginInterface.SavePluginConfig(this);
}
