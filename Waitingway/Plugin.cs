using Dalamud.IoC;
using Dalamud.Plugin;
using Dalamud.Interface.Windowing;
using Waitingway.Windows;
using Waitingway.Utils;
using System.Text.Json;

namespace Waitingway;

public sealed class Plugin : IDalamudPlugin
{
    public WindowSystem WindowSystem { get; }
    public Settings SettingsWindow { get; }
    public SettingsButton LobbyButtonWindow { get; }
    public Queue QueueWindow { get; }

    public Configuration Configuration { get; }
    public IconManager IconManager { get; }
    public Hooks Hooks { get; }
    public QueueTracker QueueTracker { get; }

    public Plugin([RequiredVersion("1.0")] DalamudPluginInterface pluginInterface)
    {
        Service.Initialize(this, pluginInterface);

        WindowSystem = new("Waitingway");
        Configuration = pluginInterface.GetPluginConfig() as Configuration ?? new();
        IconManager = new();
        Hooks = new();
        QueueTracker = new();

        SettingsWindow = new();
        LobbyButtonWindow = new();
        QueueWindow = new();

        Service.PluginInterface.UiBuilder.Draw += WindowSystem.Draw;
        Service.PluginInterface.UiBuilder.OpenConfigUi += OpenSettingsWindow;

        QueueTracker.OnRecap += recap =>
        {
            Log.Debug($"EVENT: New recap: {JsonSerializer.Serialize(recap)}");
        };

        QueueTracker.OnPositionUpdate += position =>
        {
            Log.Debug($"EVENT: New position: {position}");
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

        SettingsWindow.Dispose();
        LobbyButtonWindow.Dispose();
        QueueWindow.Dispose();

        QueueTracker.Dispose();
        Hooks.Dispose();
        IconManager.Dispose();
    }
}
