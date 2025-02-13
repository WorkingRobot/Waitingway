using Dalamud.Plugin;
using Dalamud.Interface.Windowing;
using Waitingway.Windows;
using Waitingway.Utils;
using System.Text.Json;
using Dalamud.Interface.ImGuiNotification;
using Dalamud.Game.Command;
using Waitingway.Natives;
using FFXIVClientStructs.FFXIV.Client.Game.UI;
using FFXIVClientStructs.FFXIV.Client.Game;
using Waitingway.Hooks;
using Waitingway.Api.Login;
using Waitingway.Api.Duty;

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
    public Hooks.Hooks Hooks { get; }
    public LoginQueueTracker LoginTracker { get; }
    public DutyQueueTracker DutyTracker { get; }
    public LoginNotificationTracker LoginNotificationTracker { get; }
    public DutyNotificationTracker DutyNotificationTracker { get; }
    public Api.Api Api { get; }
    public IPCProvider IPCProvider { get; }

    public Plugin(IDalamudPluginInterface pluginInterface)
    {
        Service.Initialize(this, pluginInterface);

        WindowSystem = new("Waitingway");
        Configuration = pluginInterface.GetPluginConfig() as Configuration ?? new();
        IconManager = new();
        Version = new();
        Hooks = new();
        LoginTracker = new();
        DutyTracker = new();
        Api = new();
        LoginNotificationTracker = new();
        DutyNotificationTracker = new();
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

        LoginTracker.OnBeginQueue += () =>
            Log.Debug($"EVENT: BEGIN: {JsonSerializer.Serialize(LoginTracker.CurrentRecap!, Api.JsonOptions)}");

        LoginTracker.OnUpdateQueue += () =>
            Log.Debug($"EVENT: UPDATE: {LoginTracker.CurrentRecap!.CurrentPosition!.PositionNumber}");

        LoginTracker.OnCompleteQueue += () =>
            Log.Debug($"EVENT: FINISH: {JsonSerializer.Serialize(LoginTracker.CurrentRecap!, Api.JsonOptions)}");

        LoginTracker.OnCompleteQueue += () =>
        {
            var recap = LoginTracker.CurrentRecap!;
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


        DutyTracker.OnBeginQueue += () =>
            Log.Debug($"EVENT: BEGIN: {JsonSerializer.Serialize(DutyTracker.CurrentRecap!, Api.JsonOptions)}");

        DutyTracker.OnUpdateQueue += () =>
            Log.Debug($"EVENT: UPDATE: {JsonSerializer.Serialize(DutyTracker.CurrentRecap!.LastUpdate, Api.JsonOptions)}");

        DutyTracker.OnPopQueue += () =>
            Log.Debug($"EVENT: POP: {JsonSerializer.Serialize(DutyTracker.CurrentRecap!.LastPop, Api.JsonOptions)}");

        DutyTracker.OnFinalizeQueue += () =>
            Log.Debug($"EVENT: FINALIZE: {JsonSerializer.Serialize(DutyTracker.CurrentRecap!, Api.JsonOptions)}");

        DutyTracker.OnFinalizeQueue += () =>
        {
            var recap = DutyTracker.CurrentRecap!;
            var elapsed = recap.EndTime!.Value - recap.StartTime;
            var world = World.GetWorld(recap.WorldId);
            Log.Notify(new Notification
            {
                Type = recap.Successful ? NotificationType.Success : NotificationType.Warning,
                Title = $"Queue {(recap.Successful ? "Successful" : "Unsuccessful")}",
                Content = $"Queued for {elapsed.ToString(Log.GetTimeSpanFormat(elapsed))}",
                Minimized = false
            });
        };
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
        LoginNotificationTracker.Dispose();
        DutyNotificationTracker.Dispose();
        Api.Dispose();
        LoginTracker.Dispose();
        DutyTracker.Dispose();
        Hooks.Dispose();
        IconManager.Dispose();
    }
}
