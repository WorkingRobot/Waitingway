using Dalamud.Game.ClientState.Objects;
using Dalamud.Game;
using Dalamud.Interface.Windowing;
using Dalamud.IoC;
using Dalamud.Plugin.Services;
using Dalamud.Plugin;
using Waitingway.Utils;
using Waitingway.Api.Login;
using Waitingway.Api.Duty;

namespace Waitingway;

public sealed class Service
{
#pragma warning disable CS8618 // Non-nullable field must contain a non-null value when exiting constructor. Consider declaring as nullable.
    [PluginService] public static IDalamudPluginInterface PluginInterface { get; private set; }
    [PluginService] public static ICommandManager CommandManager { get; private set; }
    [PluginService] public static IAddonEventManager AddonEventManager { get; private set; }
    [PluginService] public static IObjectTable Objects { get; private set; }
    [PluginService] public static IAddonLifecycle AddonLifecycle { get; private set; }
    [PluginService] public static INotificationManager NotificationManager { get; private set; }
    [PluginService] public static ISigScanner SigScanner { get; private set; }
    [PluginService] public static IGameGui GameGui { get; private set; }
    [PluginService] public static IClientState ClientState { get; private set; }
    [PluginService] public static IDataManager DataManager { get; private set; }
    [PluginService] public static ITextureProvider TextureProvider { get; private set; }
    [PluginService] public static ITargetManager TargetManager { get; private set; }
    [PluginService] public static ITitleScreenMenu TitleScreenMenu { get; private set; }
    [PluginService] public static ICondition Condition { get; private set; }
    [PluginService] public static IFramework Framework { get; private set; }
    [PluginService] public static IPluginLog PluginLog { get; private set; }
    [PluginService] public static IGameInteropProvider GameInteropProvider { get; private set; }

    public static Plugin Plugin { get; private set; }
    public static Configuration Configuration => Plugin.Configuration;
    public static WindowSystem WindowSystem => Plugin.WindowSystem;
    public static IconManager IconManager => Plugin.IconManager;
    public static Versioning Version => Plugin.Version;
    public static Hooks.Hooks Hooks => Plugin.Hooks;
    public static LoginQueueTracker LoginTracker => Plugin.LoginTracker;
    public static DutyQueueTracker DutyTracker => Plugin.DutyTracker;
    public static Api.Api Api => Plugin.Api;
    public static LoginNotificationTracker NotificationTracker => Plugin.LoginNotificationTracker;
#pragma warning restore CS8618

    internal static void Initialize(Plugin plugin, IDalamudPluginInterface iface)
    {
        Plugin = plugin;
        iface.Create<Service>();
    }
}
