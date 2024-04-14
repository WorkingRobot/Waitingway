using Dalamud.Configuration;
using System;

namespace Waitingway;

[Serializable]
public class Configuration : IPluginConfiguration
{
    public int Version { get; set; } = 1;

    public const string DefaultRemoteServer = "https://etheirys.waitingway.com";
    public string RemoteServer = DefaultRemoteServer;
    public string ClientId { get; init; } = Guid.NewGuid().ToString();
    public string ClientSalt { get; init; } = Guid.NewGuid().ToString().Split('-')[0];

    public void Save() =>
        Service.PluginInterface.SavePluginConfig(this);
}
