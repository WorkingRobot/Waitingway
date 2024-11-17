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

    private delegate bool StatusCodeHandlerLoginDelegate(StatusCodeHandler* handler, nint packetData);
    private delegate void AgentLobbyUpdatePositionDelegate(AgentLobby* agent, int newPosition);
    private delegate bool AgentLobbySendIdentify6Delegate(AgentLobby* agent, int characterEntryIdx);
    private delegate void LobbyUIClientReportErrorDelegate(LobbyUIClient* client, LobbyStatusCode* status);
    public delegate void DuplicateComponentNodeDelegate(AtkUldManager* manager, int componentNodeId, int duplicateCount, int nodeIdOffset);
    public delegate AtkResNode* GetDuplicatedNodeDelegate(AtkUldManager* manager, int nodeId, int idx, int offset);

    private readonly Hook<AgentLobby.Delegates.ReceiveEvent> agentLobbyReceiveEventHook = null!;

    [Signature("40 57 48 83 EC 20 66 83 7A", DetourName = nameof(StatusCodeHandlerLoginDetour))]
    private readonly Hook<StatusCodeHandlerLoginDelegate> statusCodeHandlerLoginHook = null!;

    [Signature("40 55 53 56 57 41 54 48 8D AC 24 ?? ?? ?? ?? 48 81 EC ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 85 ?? ?? ?? ?? 8B B1", DetourName = nameof(AgentLobbyUpdatePositionDetour))]
    private readonly Hook<AgentLobbyUpdatePositionDelegate> agentLobbyUpdatePositionHook = null!;

    [Signature("E8 ?? ?? ?? ?? 83 7E 20 00 4C 8B B4 24", DetourName = nameof(AgentLobbySendIdentify6Detour))]
    private readonly Hook<AgentLobbySendIdentify6Delegate> agentLobbySendIdentify6Hook = null!;

    private readonly Hook<LobbyUIClientReportErrorDelegate> lobbyUIClientReportErrorHook = null!;

    [Signature("E8 ?? ?? ?? ?? 40 38 7D 2C")]
    public readonly DuplicateComponentNodeDelegate duplicateComponentNode = null!;

    [Signature("E8 ?? ?? ?? ?? 45 8B DD")]
    public readonly GetDuplicatedNodeDelegate getDuplicatedNode = null!;

    public Hooks()
    {
        agentLobbyReceiveEventHook = Service.GameInteropProvider.HookFromAddress<AgentLobby.Delegates.ReceiveEvent>(
            (nint)AgentLobby.StaticVirtualTablePointer->ReceiveEvent,
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

    private AtkValue* AgentLobbyReceiveEventDetour(AgentLobby* @this, AtkValue* returnValue, AtkValue* values, uint valueCount, ulong eventKind)
    {
        var agent2 = (AgentLobby2*)@this;
        if (valueCount > 0)
        {
            switch (eventKind)
            {
                case 0x03:
                    // 0 = OK
                    // 1 = Cancel
                    if (values[0].Int == 0)
                    {
                        var entry = @this->LobbyData.CharaSelectEntries[agent2->SelectedCharacterIndex].Value;
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

        return agentLobbyReceiveEventHook.Original(@this, returnValue, values, valueCount, eventKind);
    }

    public static long? AgentLobbyGetTimeSinceLastIdentify()
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
