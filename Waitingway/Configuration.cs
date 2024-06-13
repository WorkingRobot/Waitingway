using Dalamud.Configuration;
using Newtonsoft.Json;
using System;
using static Waitingway.Utils.QueueTracker;
using System.Collections.Generic;

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

    [JsonProperty("LastFailedRecaps_ProbablyDoNotShareThisWithAnyoneEither")]
    private Dictionary<ulong, Recap> LastFailedRecaps { get; init; } = [];

    public EstimatorType Estimator { get; set; } = EstimatorType.Geometric;
    public float IdentifyLatency { get; set; } = 0.6f;
    public float LoginLatency { get; set; } = 1.25f;
    public float DefaultRate { get; set; } = 100;
    public bool HideIdentifyTimer { get; set; }
    public int NotificationThreshold { get; set; }

    public void AddFailedRecap(Recap recap)
    {
        LastFailedRecaps[recap.CharacterContentId] = recap;
        Save();
    }

    public Recap? TakeFailedRecap(ulong contentId)
    {
        if (LastFailedRecaps.Remove(contentId, out var recap))
        {
            Save();
            return recap;
        }
        return null;
    }

    public void Save() =>
        Service.PluginInterface.SavePluginConfig(this);
}
