using Dalamud.Plugin;
using Dalamud.Interface.Windowing;
using Waitingway.Windows;
using Waitingway.Utils;
using System.Text.Json;
using Dalamud.Interface.ImGuiNotification;
using Dalamud.Game.Command;
using Waitingway.Natives;

namespace Waitingway;

public sealed class Plugin : IDalamudPlugin
{
    public WindowSystem WindowSystem { get; }
    public Settings SettingsWindow { get; }
    public Queue QueueWindow { get; }

    public CharaListMenu SettingsButton { get; }
    public WorldSelector WorldSelector { get; }

    public Configuration Configuration { get; }
    public IconManager IconManager { get; }
    public Versioning Version { get; }
    public Hooks Hooks { get; }
    public QueueTracker QueueTracker { get; }
    public Api Api { get; }
    public NotificationTracker NotificationTracker { get; }
    public IPCProvider IPCProvider { get; }

    public Plugin(IDalamudPluginInterface pluginInterface)
    {
        Service.Initialize(this, pluginInterface);

        WindowSystem = new("Waitingway");
        Configuration = pluginInterface.GetPluginConfig() as Configuration ?? new();
        IconManager = new();
        Version = new();
        Hooks = new();
        QueueTracker = new();
        Api = new();
        NotificationTracker = new();
        IPCProvider = new();

        SettingsWindow = new();
        QueueWindow = new();

        SettingsButton = new();
        WorldSelector = new();

        Service.TitleScreenMenu.AddEntry("Waitingway Settings", IconManager.GetAssemblyTextureCached("Graphics.menu_icon.png").GetWrap(), () => OpenSettingsWindow(true));

        Service.PluginInterface.UiBuilder.Draw += WindowSystem.Draw;
        Service.PluginInterface.UiBuilder.OpenConfigUi += () => OpenSettingsWindow();

        Service.CommandManager.AddHandler("/waitingway", new CommandInfo((_, _) => OpenSettingsWindow(true))
        {
            HelpMessage = "Open the Waitingway settings window."
        });

        QueueTracker.OnBeginQueue += () =>
            Log.Debug($"EVENT: BEGIN: {JsonSerializer.Serialize(QueueTracker.CurrentRecap!, Api.JsonOptions)}");

        QueueTracker.OnUpdateQueue += () =>
            Log.Debug($"EVENT: UPDATE: {QueueTracker.CurrentRecap!.CurrentPosition!.PositionNumber}");

        QueueTracker.OnCompleteQueue += () =>
            Log.Debug($"EVENT: FINISH: {JsonSerializer.Serialize(QueueTracker.CurrentRecap!, Api.JsonOptions)}");

        QueueTracker.OnCompleteQueue += () =>
        {
            var recap = QueueTracker.CurrentRecap!;
            var elapsed = recap.EndTime - recap.StartTime;
            var world = World.GetWorld(recap.WorldId);
            Log.Notify(new Notification
            {
                Type = recap.Successful ? NotificationType.Success : NotificationType.Warning,
                Title = $"Queue {(recap.Successful ? "Successful" : "Unsuccessful")}",
                MinimizedText = $"Queued for {elapsed.ToString(Log.GetTimeSpanFormat(elapsed))}",
                Content = $"Queued for {elapsed.ToString(Log.GetTimeSpanFormat(elapsed))} for {world?.WorldName ?? "Unknown"} ({world?.DatacenterName ?? "Unknown"})",
                Minimized = false
            });
        };

        Log.Debug(JsonSerializer.Serialize(World.GetWorlds()));
    }

    public void OpenSettingsWindow(bool force = false)
    {
        if (SettingsWindow.IsOpen ^= force ? !SettingsWindow.IsOpen : true)
            SettingsWindow.BringToFront();
    }

    public void OpenSettingsTab(string selectedTabLabel)
    {
        OpenSettingsWindow(true);
        SettingsWindow.SelectTab(selectedTabLabel);
    }

    public void Dispose()
    {
        Service.CommandManager.RemoveHandler("/waitingway");

        Configuration.Save();

        SettingsButton.Dispose();
        WorldSelector.Dispose();

        QueueWindow.Dispose();
        SettingsWindow.Dispose();

        IPCProvider.Dispose();
        NotificationTracker.Dispose();
        Api.Dispose();
        QueueTracker.Dispose();
        Hooks.Dispose();
        IconManager.Dispose();
    }
}
