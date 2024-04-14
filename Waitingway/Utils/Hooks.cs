using Dalamud.Hooking;
using Dalamud.Utility.Signatures;
using FFXIVClientStructs.FFXIV.Client.UI.Agent;
using FFXIVClientStructs.FFXIV.Component.GUI;
using System;
using System.Runtime.InteropServices;

namespace Waitingway.Utils;

public sealed unsafe class Hooks : IDisposable
{
    [StructLayout(LayoutKind.Explicit, Size = 0x20)]
    private struct LobbyStatusUpdateData
    {
        [FieldOffset(0x18)] public int StatusCode;
        [FieldOffset(0x1C)] public int QueuePosition;
    }

    public event Action? OnEnterQueue;
    public event Action<bool>? OnExitQueue; // true => successful, false => cancelled/left
    public event Action<int>? OnNewQueuePosition;

    private delegate void* AgentLobbyReceiveEventDelegate(AgentLobby* agent, void* eventData, AtkValue* values, uint valueCount, ulong eventKind);

    private readonly Hook<AgentLobbyReceiveEventDelegate> agentLobbyReceiveEventHook = null!;

    private delegate nint LobbyStatusUpdateDelegate(nint a1, LobbyStatusUpdateData* data);

    [Signature("48 89 5C 24 ?? 57 48 81 EC ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 84 24 ?? ?? ?? ?? 8B 41 10 48 8D 7A 10", DetourName = nameof(LobbyStatusUpdateDetour))]
    private readonly Hook<LobbyStatusUpdateDelegate> lobbyStatusUpdateHook = null!;

    private delegate void LobbyRequestCallbackOnLoginDelegate(LobbyUIClient* client);

    private readonly Hook<LobbyRequestCallbackOnLoginDelegate> lobbyRequestCallbackOnLoginHook = null!;

    public Hooks()
    {
        agentLobbyReceiveEventHook = Service.GameInteropProvider.HookFromAddress<AgentLobbyReceiveEventDelegate>(
            (nint)((AgentInterface.AgentInterfaceVTable*)AgentLobby.StaticAddressPointers.VTable)->ReceiveEvent,
            AgentLobbyReceiveEventDetour);

        // vf22
        lobbyRequestCallbackOnLoginHook = Service.GameInteropProvider.HookFromAddress<LobbyRequestCallbackOnLoginDelegate>(
            ((nint*)LobbyUIClient.StaticAddressPointers.VTable)[22],
            LobbyRequestCallbackOnLoginDetour);

        Service.GameInteropProvider.InitializeFromAttributes(this);

        agentLobbyReceiveEventHook.Enable();
        lobbyStatusUpdateHook.Enable();
        lobbyRequestCallbackOnLoginHook.Enable();
    }

    private void* AgentLobbyReceiveEventDetour(AgentLobby* agent, void* eventData, AtkValue* values, uint valueCount, ulong eventKind)
    {
        if (valueCount > 0 && eventKind != 0)
        {
            switch (eventKind)
            {
                case 0x03:
                    // 0 = OK
                    // 1 = Cancel
                    if (values[0].Int == 0)
                        OnEnterQueue?.Invoke();
                    break;
                case 0x1C:
                    // 0 = OK
                    // 1 = Cancel
                    if (values[0].Int == 0)
                        OnExitQueue?.Invoke(false);
                    break;
            }
        }

        return agentLobbyReceiveEventHook.Original(agent, eventData, values, valueCount, eventKind);
    }

    private nint LobbyStatusUpdateDetour(nint a1, LobbyStatusUpdateData* data)
    {
        if (data->StatusCode == 1007) // World Full
        {
            if (data->QueuePosition > 0)
                OnNewQueuePosition?.Invoke(data->QueuePosition);
            // < 0 means "Character not properly logged off"
        }

        return lobbyStatusUpdateHook.Original(a1, data);
    }

    private void LobbyRequestCallbackOnLoginDetour(LobbyUIClient* client)
    {
        OnExitQueue?.Invoke(true);
        lobbyRequestCallbackOnLoginHook.Original(client);
    }

    public void Dispose()
    {
        agentLobbyReceiveEventHook?.Dispose();
        lobbyStatusUpdateHook?.Dispose();
        lobbyRequestCallbackOnLoginHook?.Dispose();
    }
}
