using Dalamud.IoC;
using Dalamud.Plugin;
using Dalamud.Interface.Windowing;
using Waitingway.Windows;
using Waitingway.Utils;
using System.Text.Json;
using Dalamud.Interface.ImGuiNotification;
using Dalamud.Interface.Internal.Notifications;

namespace Waitingway;

public sealed class Plugin : IDalamudPlugin
{
    public WindowSystem WindowSystem { get; }
    public Settings SettingsWindow { get; }
    public SettingsButton LobbyButtonWindow { get; }
    public Queue QueueWindow { get; }

    public Configuration Configuration { get; }
    public IconManager IconManager { get; }
    public Versioning Version { get; }
    public Hooks Hooks { get; }
    public QueueTracker QueueTracker { get; }
    public Api Api { get; }
    public NotificationTracker NotificationTracker { get; }

    public Plugin([RequiredVersion("1.0")] DalamudPluginInterface pluginInterface)
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

        SettingsWindow = new();
        LobbyButtonWindow = new();
        QueueWindow = new();

        Service.PluginInterface.UiBuilder.Draw += WindowSystem.Draw;
        Service.PluginInterface.UiBuilder.OpenConfigUi += OpenSettingsWindow;

        QueueTracker.OnBeginQueue += recap =>
            Log.Debug($"EVENT: BEGIN: {JsonSerializer.Serialize(recap, Api.JsonOptions)}");

        QueueTracker.OnUpdateQueue += recap =>
            Log.Debug($"EVENT: UPDATE: {recap.CurrentPosition!.PositionNumber}");

        QueueTracker.OnCompleteQueue += recap =>
            Log.Debug($"EVENT: FINISH: {JsonSerializer.Serialize(recap, Api.JsonOptions)}");

        QueueTracker.OnCompleteQueue += recap =>
        {
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
    }

    public void OpenSettingsWindow()
    {
        if (SettingsWindow.IsOpen ^= true)
            SettingsWindow.BringToFront();
    }

    public void OpenSettingsTab(string selectedTabLabel)
    {
        OpenSettingsWindow();
        SettingsWindow.SelectTab(selectedTabLabel);
    }

    public void Dispose()
    {
        Configuration.Save();

        QueueWindow.Dispose();
        LobbyButtonWindow.Dispose();
        SettingsWindow.Dispose();

        NotificationTracker.Dispose();
        Api.Dispose();
        QueueTracker.Dispose();
        Hooks.Dispose();
        IconManager.Dispose();
    }
}
