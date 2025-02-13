using Dalamud.Plugin;
using System;
using System.Collections.Generic;
using System.Reflection;
using Waitingway.Api.Duty;
using Waitingway.Api.Login;

namespace Waitingway.Utils;

public sealed class IPCProvider : IDisposable
{
    [AttributeUsage(AttributeTargets.Method | AttributeTargets.Property)]
    private sealed class IPCGateAttribute : Attribute { }

    private List<object> CallGateProviders { get; } = [];

    public IPCProvider()
    {
        var propProviderMethod = typeof(IDalamudPluginInterface).GetMethod("GetIpcProvider", 1, [typeof(string)])
            ?? throw new InvalidOperationException("GetIpcProvider method not found");

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
            
            var registerFunc = callGateProvider.GetType().GetMethod("RegisterFunc")
                ?? throw new InvalidOperationException("RegisterFunc method not found");

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
        Duty,
    }

    private static IPCQueueType BaseQueueType
    {
        get
        {
            if (Service.LoginTracker.CurrentState != LoginQueueTracker.QueueState.NotQueued)
                return IPCQueueType.Login;
            if (Service.DutyTracker.CurrentState != DutyQueueTracker.QueueState.NotQueued)
                return IPCQueueType.Duty;
            return IPCQueueType.None;
        }
    }

    private static int? GetPosition() =>
        BaseQueueType switch
        {
            IPCQueueType.Login => Service.LoginTracker.CurrentRecap?.CurrentPosition?.PositionNumber,
            IPCQueueType.Duty => null,
            _ => null
        };

    private static DateTime? GetStartTime() =>
        BaseQueueType switch
        {
            IPCQueueType.Login => Service.LoginTracker.CurrentRecap?.StartTime,
            IPCQueueType.Duty => null,
            _ => null
        };

    private static DateTime? GetEstimatedEndTime() =>
        BaseQueueType switch
        {
            IPCQueueType.Login => Service.LoginTracker.CurrentRecap?.EstimatedEndTime,
            IPCQueueType.Duty => null,
            _ => null
        };

    [IPCGate]
    public int? QueueType =>
        BaseQueueType == IPCQueueType.None
        ? null
        : (int)BaseQueueType;

    [IPCGate]
    public int? CurrentPosition =>
        GetPosition();

    [IPCGate]
    public TimeSpan? ElapsedTime =>
        GetStartTime() is { } startTime
        ? DateTime.UtcNow - startTime
        : null;

    [IPCGate]
    public TimeSpan? EstimatedTimeRemaining
    {
        get
        {
            if (GetEstimatedEndTime() is { } endTime)
            {
                var now = DateTime.UtcNow;
                if (endTime > now)
                    return endTime - now;
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
