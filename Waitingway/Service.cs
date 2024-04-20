using Dalamud.Game.ClientState.Objects;
using Dalamud.Game;
using Dalamud.Interface.Windowing;
using Dalamud.IoC;
using Dalamud.Plugin.Services;
using Dalamud.Plugin;
using Waitingway.Utils;

namespace Waitingway;

public sealed class Service
{
#pragma warning disable CS8618 // Non-nullable field must contain a non-null value when exiting constructor. Consider declaring as nullable.
    [PluginService] public static DalamudPluginInterface PluginInterface { get; private set; }
    [PluginService] public static ICommandManager CommandManager { get; private set; }
    [PluginService] public static IObjectTable Objects { get; private set; }
    [PluginService] public static IAddonLifecycle AddonLifecycle { get; private set; }
    [PluginService] public static INotificationManager NotificationManager { get; private set; }
    [PluginService] public static ISigScanner SigScanner { get; private set; }
    [PluginService] public static IGameGui GameGui { get; private set; }
    [PluginService] public static IClientState ClientState { get; private set; }
    [PluginService] public static IDataManager DataManager { get; private set; }
    [PluginService] public static ITextureProvider TextureProvider { get; private set; }
    [PluginService] public static ITargetManager TargetManager { get; private set; }
    [PluginService] public static ICondition Condition { get; private set; }
    [PluginService] public static IFramework Framework { get; private set; }
    [PluginService] public static IPluginLog PluginLog { get; private set; }
    [PluginService] public static IGameInteropProvider GameInteropProvider { get; private set; }

    public static Plugin Plugin { get; private set; }
    public static Configuration Configuration => Plugin.Configuration;
    public static WindowSystem WindowSystem => Plugin.WindowSystem;
    public static IconManager IconManager => Plugin.IconManager;
    public static Hooks Hooks => Plugin.Hooks;
    public static QueueTracker QueueTracker => Plugin.QueueTracker;
    public static Api Api => Plugin.Api;
    public static NotificationTracker NotificationTracker => Plugin.NotificationTracker;
#pragma warning restore CS8618

    internal static void Initialize(Plugin plugin, DalamudPluginInterface iface)
    {
        Plugin = plugin;
        iface.Create<Service>();
    }
}
