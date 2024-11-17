using Dalamud.Plugin;
using Dalamud.Interface.Windowing;
using Waitingway.Windows;
using Waitingway.Utils;
using System.Text.Json;
using Dalamud.Interface.ImGuiNotification;
using Dalamud.Game.Command;
using Waitingway.Natives;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using System.Runtime.InteropServices;
using FFXIVClientStructs.FFXIV.Common.Component.Excel;
using FFXIVClientStructs.FFXIV.Client.Network;
using FFXIVClientStructs.STD;
using FFXIVClientStructs.FFXIV.Component.Text;
using FFXIVClientStructs.FFXIV.Client.System.String;

namespace Waitingway;

[StructLayout(LayoutKind.Explicit, Size = 0x1E68)]
public unsafe partial struct AgentLobby2
{
    [FieldOffset(0x40)] public LobbyData LobbyData; // for lack of a better name
    [FieldOffset(0x48)] public LobbySubscriptionInfo* SubscriptionInfo;

    [FieldOffset(0xA20)] public ExcelSheet* ErrorSheet;
    [FieldOffset(0xA28)] public ExcelSheet* LobbySheet;
    [FieldOffset(0xA30)] public NetworkModuleProxy* NetworkModuleProxy;
    [FieldOffset(0xA38)] public StdDeque<TextParameter> LobbyTextParameters;
    //[FieldOffset(0xA60), FixedSizeArray] internal FixedSizeArray4<Utf8String> _tempUtf8Strings;
    [FieldOffset(0xC00)] public Utf8String ConnectingToDatacenterString;
    [FieldOffset(0xC68)] public StdVector<Utf8String> VersionStrings;
    [FieldOffset(0xC80)] public Utf8String DisplayedVersionString;

    //[FieldOffset(0xD00), FixedSizeArray] internal FixedSizeArray8<Utf8String> _unkUtf8Strings;

    [FieldOffset(0x1178)] public sbyte ServiceAccountIndex;
    [FieldOffset(0x1179)] public byte SelectedCharacterIndex;

    [FieldOffset(0x1180)] public ulong HoveredCharacterContentId;
    [FieldOffset(0x1188)] public byte DataCenter;

    [FieldOffset(0x118A)] public short WorldIndex; // index in CurrentDataCenterWorlds
    [FieldOffset(0x118C)] public ushort WorldId;

    [FieldOffset(0x11F8)] public uint DialogAddonId;
    [FieldOffset(0x1194)] public uint DialogAddonId2;
    [FieldOffset(0x1200)] public uint LobbyScreenTextAddonId;
    [FieldOffset(0x119C)] public uint LogoAddonId;
    [FieldOffset(0x11A0)] public uint TitleDCWorldMapAddonId;
    [FieldOffset(0x11A4)] public uint TitleMovieSelectorAddonId;
    [FieldOffset(0x11A8)] public uint TitleGameVersionAddonId;
    [FieldOffset(0x11AC)] public uint TitleConnectAddonId;
    [FieldOffset(0x11B0)] public uint CharaSelectAddonId;
    [FieldOffset(0x11B4)] public uint CharaMakeDataImportAddonId;
    [FieldOffset(0x11B8)] public uint LoadPreviouslySavedAppearanceDataDialogAddonId; // SelectYesno
    [FieldOffset(0x11BC)] public uint LoadSavedCharacterCreationDataDialogAddonId; // SelectYesno
    [FieldOffset(0x11C0)] public uint CreateNewCharacterDialogAddonId; // SelectYesno
    [FieldOffset(0x11C4)] public uint LobbyWKTAddonId;

    [FieldOffset(0x11D4)] public byte LobbyUpdateStage;

    [FieldOffset(0x11D7)] public byte LobbyUIStage;

    [FieldOffset(0x11E0)] public long IdleTime;

    [FieldOffset(0x11F0)] public long QueueTimeSinceLastUpdate;
    [FieldOffset(0x11F8)] public int QueuePosition;

    [FieldOffset(0x11FD)] public sbyte HoveredCharacterIndex; // index in CharaSelectCharacterList

    [FieldOffset(0x1200)] public ulong SelectedCharacterContentId;

    [FieldOffset(0x1210)] public bool IsLoggedIn; // set in ProcessPacketPlayerSetup, unset in LogoutCallbackInterface_OnLogout
    [FieldOffset(0x1211)] public bool IsLoggedIntoZone; // set in ZoneLoginCallbackInterface_OnZoneLogin (+0x38)

    [FieldOffset(0x1213)] public bool LogoutShouldCloseGame;

    [FieldOffset(0x1310)] public bool TemporaryLocked; // "Please wait and try logging in later."

    [FieldOffset(0x1328)] public ulong RequestContentId;

    [FieldOffset(0x1E84)] public bool HasShownCharacterNotFound; // "The character you last logged out with in this play environment could not be found on the current data center."

    //[FieldOffset(0x1348)] public LogoutCallbackInterface.LogoutParams LogoutParams;
}

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

        //Log.Debug(JsonSerializer.Serialize(World.GetWorlds()));
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
