using Dalamud.Plugin;
using System;
using System.Collections.Generic;
using System.Reflection;

namespace Waitingway.Utils;

public sealed class IPCProvider : IDisposable
{
    [AttributeUsage(AttributeTargets.Method | AttributeTargets.Property)]
    private sealed class IPCGateAttribute : Attribute { }

    private List<object> CallGateProviders { get; } = [];

    public IPCProvider()
    {
        var propProviderMethod = typeof(DalamudPluginInterface).GetMethod("GetIpcProvider", 1, [typeof(string)]);
        if (propProviderMethod is null)
            throw new InvalidOperationException("GetIpcProvider method not found");

        foreach (var prop in typeof(IPCProvider).GetProperties(BindingFlags.Instance | BindingFlags.Public))
        {
            if (prop.GetCustomAttribute<IPCGateAttribute>() is null)
                continue;

            if (prop.GetMethod is null)
                throw new InvalidOperationException("Property must have a getter");

            var type = prop.PropertyType;

            var callGateProvider = propProviderMethod.MakeGenericMethod(type).Invoke(Service.PluginInterface, [$"Waitingway.{prop.Name}"]);

            if (callGateProvider is null)
                throw new InvalidOperationException("CallGateProvider is null");
            
            var registerFunc = callGateProvider.GetType().GetMethod("RegisterFunc");
            if (registerFunc is null)
                throw new InvalidOperationException("RegisterFunc method not found");

            registerFunc.Invoke(callGateProvider, [Delegate.CreateDelegate(registerFunc.GetParameters()[0].ParameterType, this, prop.GetMethod)]);

            CallGateProviders.Add(callGateProvider);

            Log.Debug($"Bound {prop.Name} ({type}) to IPC");
        }
    }

    private enum IPCQueueType
    {
        None,
        Login,
        DatacenterTravel,
        WorldTravel,
        Roulette,
    }

    private IPCQueueType BaseQueueType
    {
        get
        {
            if (Service.QueueTracker.CurrentState != QueueTracker.QueueState.NotQueued)
                return IPCQueueType.Login;
            return IPCQueueType.None;
        }
    }

    [IPCGate]
    public int? QueueType => BaseQueueType == IPCQueueType.None ? null : (int)BaseQueueType;

    [IPCGate]
    public int? CurrentPosition => Service.QueueTracker.CurrentRecap?.CurrentPosition?.PositionNumber;

    [IPCGate]
    public TimeSpan? ElapsedTime => Service.QueueTracker.CurrentRecap?.StartTime is { } startTime ? DateTime.UtcNow - startTime : null;

    [IPCGate]
    public TimeSpan? EstimatedTimeRemaining {
        get {
            var ret = Service.QueueTracker.CurrentRecap?.EstimatedEndTime - DateTime.UtcNow;
            if (ret is { } timeSpan)
            {
                if (timeSpan.Ticks > 0)
                    return timeSpan;
                else
                    return TimeSpan.Zero;
            }
            return null;
        }
    }
    
    public void Dispose()
    {
        foreach(var provider in CallGateProviders)
            provider.GetType().GetMethod("UnregisterFunc")!.Invoke(provider, []);
    }
}
