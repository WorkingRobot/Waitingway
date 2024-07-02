using Dalamud.Hooking;
using Dalamud.Utility.Signatures;
using FFXIVClientStructs.FFXIV.Client.System.String;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Component.GUI;
using System;
using System.Runtime.InteropServices;

namespace Waitingway.Utils;

public sealed unsafe class Hooks : IDisposable
{
    [StructLayout(LayoutKind.Explicit, Size = 0x40)]
    public unsafe struct StatusCodeHandler
    {

    }

    [StructLayout(LayoutKind.Explicit, Size = 0x80)]
    public unsafe struct LobbyStatusCode
    {
        [FieldOffset(0x00)] public int Code;
        [FieldOffset(0x08)] public int CodeType;
        [FieldOffset(0x10)] public Utf8String String;
        [FieldOffset(0x78)] public ushort ErrorSheetRow;
    }

    [StructLayout(LayoutKind.Explicit, Size = 0x1DF8)]
    public unsafe partial struct AgentLobby2
    {
        [FieldOffset(0x48)] public LobbySubscriptionInfo* SubscriptionInfo;

        [FieldOffset(0x1164)] public byte LobbyUpdateStage;

        [FieldOffset(0x1111)] public byte SelectedCharacterIndex;

        [FieldOffset(0x1180)] public ulong QueueTimeSinceLastUpdate;
    }

    public event Action<string, ulong, bool, ushort, ushort>? OnEnterQueue; // characterName, contentId, isFreeTrial, homeWorldId, worldId
    public event Action? OnCancelQueue; // Manually cancelled
    public event Action<int, int, string, ushort>? OnFailedQueue; // Error code for queue: codeType, code, codeString, errorSheetRow
    public event Action? OnExitQueue; // Exited queue (logged in)
    public event Action? OnSendIdentify; // Identify sent
    public event Action<int>? OnNewQueuePosition; // New position

    private delegate void* AgentLobbyReceiveEventDelegate(AgentLobby* agent, void* eventData, AtkValue* values, uint valueCount, ulong eventKind);
    private delegate bool StatusCodeHandlerLoginDelegate(StatusCodeHandler* handler, nint packetData);
    private delegate void AgentLobbyUpdatePositionDelegate(AgentLobby* agent, int newPosition);
    private delegate bool AgentLobbySendIdentify6Delegate(AgentLobby* agent, int characterEntryIdx);
    private delegate void LobbyUIClientReportErrorDelegate(LobbyUIClient* client, LobbyStatusCode* status);

    private readonly Hook<AgentLobbyReceiveEventDelegate> agentLobbyReceiveEventHook = null!;

    [Signature("40 57 48 83 EC 20 66 83 7A", DetourName = nameof(StatusCodeHandlerLoginDetour))]
    private readonly Hook<StatusCodeHandlerLoginDelegate> statusCodeHandlerLoginHook = null!;

    [Signature("40 55 53 56 57 41 54 48 8D AC 24 ?? ?? ?? ?? 48 81 EC ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 85 ?? ?? ?? ?? 8B B1", DetourName = nameof(AgentLobbyUpdatePositionDetour))]
    private readonly Hook<AgentLobbyUpdatePositionDelegate> agentLobbyUpdatePositionHook = null!;

    [Signature("E8 ?? ?? ?? ?? 83 7E 20 00 4C 8B BC 24", DetourName = nameof(AgentLobbySendIdentify6Detour))]
    private readonly Hook<AgentLobbySendIdentify6Delegate> agentLobbySendIdentify6Hook = null!;

    private readonly Hook<LobbyUIClientReportErrorDelegate> lobbyUIClientReportErrorHook = null!;

    public Hooks()
    {
        agentLobbyReceiveEventHook = Service.GameInteropProvider.HookFromAddress<AgentLobbyReceiveEventDelegate>(
            (nint)(AgentLobby.StaticVirtualTablePointer->ReceiveEvent),
            AgentLobbyReceiveEventDetour);

        lobbyUIClientReportErrorHook = Service.GameInteropProvider.HookFromAddress<LobbyUIClientReportErrorDelegate>(
            ((nint*)LobbyUIClient.StaticVirtualTablePointer)[4],
            LobbyUIClientReportErrorDetour);

        Service.GameInteropProvider.InitializeFromAttributes(this);

        agentLobbyReceiveEventHook.Enable(); // for login start and premature cancels
        statusCodeHandlerLoginHook.Enable();
        agentLobbyUpdatePositionHook.Enable();
        agentLobbySendIdentify6Hook.Enable();
        lobbyUIClientReportErrorHook.Enable();
    }

    private void LobbyUIClientReportErrorDetour(LobbyUIClient* client, LobbyStatusCode* status)
    {
        OnFailedQueue?.Invoke(status->CodeType, status->Code, status->String.ToString(), status->ErrorSheetRow);
        lobbyUIClientReportErrorHook.Original(client, status);
    }

    private void AgentLobbyUpdatePositionDetour(AgentLobby* agent, int newPosition)
    {
        OnNewQueuePosition?.Invoke(newPosition);
        agentLobbyUpdatePositionHook.Original(agent, newPosition);
    }
    
    private bool AgentLobbySendIdentify6Detour(AgentLobby* agent, int characterEntryIdx)
    {
        OnSendIdentify?.Invoke();
        return agentLobbySendIdentify6Hook.Original(agent, characterEntryIdx);
    }

    private bool StatusCodeHandlerLoginDetour(StatusCodeHandler* handler, nint packetData)
    {
        OnExitQueue?.Invoke();
        return statusCodeHandlerLoginHook.Original(handler, packetData);
    }

    private void* AgentLobbyReceiveEventDetour(AgentLobby* agent, void* eventData, AtkValue* values, uint valueCount, ulong eventKind)
    {
        var agent2 = (AgentLobby2*)agent;
        if (valueCount > 0)
        {
            switch (eventKind)
            {
                case 0x03:
                    // 0 = OK
                    // 1 = Cancel
                    if (values[0].Int == 0)
                    {
                        var entry = agent->LobbyData.CharaSelectEntries[agent2->SelectedCharacterIndex].Value;
                        OnEnterQueue?.Invoke(
                            entry->NameString,
                            entry->ContentId,
                            (agent2->SubscriptionInfo->Flags & 0x10000000) != 0,
                            entry->HomeWorldId,
                            entry->CurrentWorldId);
                    }
                    break;
                case 0x1C:
                    // 0 = OK
                    // 1 = Cancel
                    if (values[0].Int == 0)
                        OnCancelQueue?.Invoke();
                    break;
            }
        }

        return agentLobbyReceiveEventHook.Original(agent, eventData, values, valueCount, eventKind);
    }

    public static ulong? AgentLobbyGetTimeSinceLastIdentify()
    {
        var agent = (AgentLobby2*)AgentLobby.Instance();
        if (agent->LobbyUpdateStage != 31)
            return null;
        return agent->QueueTimeSinceLastUpdate;
    }

    public void Dispose()
    {
        agentLobbyReceiveEventHook?.Dispose();
        statusCodeHandlerLoginHook?.Dispose();
        agentLobbyUpdatePositionHook?.Dispose();
        agentLobbySendIdentify6Hook?.Dispose();
        lobbyUIClientReportErrorHook?.Dispose();
    }
}
