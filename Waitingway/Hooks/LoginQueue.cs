using Dalamud.Hooking;
using Dalamud.Utility.Signatures;
using FFXIVClientStructs.FFXIV.Application.Network.LobbyClient;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Component.GUI;
using System;
using System.Runtime.InteropServices;

namespace Waitingway.Hooks;

public sealed unsafe class LoginQueue : IDisposable
{
    public delegate void EnterQueueDelegate(string characterName, ulong contentId, bool isFreeTrial, ushort homeWorldId, ushort worldId);
    public delegate void CancelQueueDelegate();
    public delegate void FailedQueueDelegate(int codeType, int code, string codeString, ushort errorSheetRow);
    public delegate void ExitQueueDelegate();
    public delegate void SendIdentifyDelegate();
    public delegate void NewQueuePositionDelegate(int newPosition);

    public event EnterQueueDelegate? OnEnterQueue;
    public event CancelQueueDelegate? OnCancelQueue; // Manually cancelled
    public event FailedQueueDelegate? OnFailedQueue;
    public event ExitQueueDelegate? OnExitQueue; // Exited queue (logged in)
    public event SendIdentifyDelegate? OnSendIdentify; // Identify sent
    public event NewQueuePositionDelegate? OnNewQueuePosition; // New position

    [StructLayout(LayoutKind.Explicit, Size = 0x40)]
    public struct StatusCodeHandler
    {

    }

    private delegate bool StatusCodeHandlerLoginDelegate(StatusCodeHandler* handler, nint packetData);

    private readonly Hook<AgentLobby.Delegates.ReceiveEvent> agentLobbyReceiveEventHook = null!;

    // vf1 at vtable located at "50 78 D3 41 01 00"
    // vf1 probably detaches the status code handler
    // Probably called GameLoginOperation or LobbyLoginOperation from classinformer
    [Signature("40 53 48 83 EC 20 66 83 7A", DetourName = nameof(StatusCodeHandlerLoginDetour))]
    private readonly Hook<StatusCodeHandlerLoginDelegate> statusCodeHandlerLoginHook = null!;

    private readonly Hook<AgentLobby.Delegates.UpdateLoginPosition> agentLobbyUpdatePositionHook = null!;

    private readonly Hook<AgentLobby.Delegates.SendLoginRequestPacket> agentLobbySendLoginRequestPacketHook = null!;

    private readonly Hook<LobbyUIClient.Delegates.ReportError> lobbyUIClientReportErrorHook = null!;

    public LoginQueue()
    {
        agentLobbyReceiveEventHook = Service.GameInteropProvider.HookFromAddress<AgentLobby.Delegates.ReceiveEvent>(
            (nint)AgentLobby.StaticVirtualTablePointer->ReceiveEvent,
            AgentLobbyReceiveEventDetour);

        agentLobbyUpdatePositionHook = Service.GameInteropProvider.HookFromAddress<AgentLobby.Delegates.UpdateLoginPosition>(
            AgentLobby.Addresses.UpdateLoginPosition.Value,
            AgentLobbyUpdatePositionDetour);

        agentLobbySendLoginRequestPacketHook = Service.GameInteropProvider.HookFromAddress<AgentLobby.Delegates.SendLoginRequestPacket>(
            AgentLobby.Addresses.SendLoginRequestPacket.Value,
            AgentLobbySendLoginRequestPacketDetour);

        lobbyUIClientReportErrorHook = Service.GameInteropProvider.HookFromAddress<LobbyUIClient.Delegates.ReportError>(
            (nint)LobbyUIClient.StaticVirtualTablePointer->ReportError,
            LobbyUIClientReportErrorDetour);

        Service.GameInteropProvider.InitializeFromAttributes(this);

        agentLobbyReceiveEventHook.Enable(); // for login start and premature cancels
        statusCodeHandlerLoginHook.Enable();
        agentLobbyUpdatePositionHook.Enable();
        agentLobbySendLoginRequestPacketHook.Enable();
        lobbyUIClientReportErrorHook.Enable();
    }

    public static long? AgentLobbyGetTimeSinceLastIdentify()
    {
        var agent = AgentLobby.Instance();
        if (agent->LobbyUpdateStage != 31)
            return null;
        return agent->QueueTimeSinceLastUpdate;
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

    private bool AgentLobbySendLoginRequestPacketDetour(AgentLobby* agent, int characterEntryIdx)
    {
        OnSendIdentify?.Invoke();
        return agentLobbySendLoginRequestPacketHook.Original(agent, characterEntryIdx);
    }

    private bool StatusCodeHandlerLoginDetour(StatusCodeHandler* handler, nint packetData)
    {
        OnExitQueue?.Invoke();
        return statusCodeHandlerLoginHook.Original(handler, packetData);
    }

    private AtkValue* AgentLobbyReceiveEventDetour(AgentLobby* @this, AtkValue* returnValue, AtkValue* values, uint valueCount, ulong eventKind)
    {
        if (valueCount > 0)
        {
            switch (eventKind)
            {
                case 0x03:
                    // 0 = OK
                    // 1 = Cancel
                    if (values[0].Int == 0)
                    {
                        var entry = @this->LobbyData.CharaSelectEntries[@this->SelectedCharacterIndex].Value;
                        OnEnterQueue?.Invoke(
                            entry->NameString,
                            entry->ContentId,
                            (@this->LobbyData.LobbyUIClient.SubscriptionInfo->Flags & 0x10000000) != 0,
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

        return agentLobbyReceiveEventHook.Original(@this, returnValue, values, valueCount, eventKind);
    }

    public void Dispose()
    {
        agentLobbyReceiveEventHook?.Dispose();
        statusCodeHandlerLoginHook?.Dispose();
        agentLobbyUpdatePositionHook?.Dispose();
        agentLobbySendLoginRequestPacketHook?.Dispose();
        lobbyUIClientReportErrorHook?.Dispose();
    }
}
