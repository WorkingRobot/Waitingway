using System;

namespace Waitingway.Hooks;

public sealed unsafe class Hooks : IDisposable
{
    public AtkHooks Atk { get; } = new();

    public DutyQueue Duty { get; } = new();
    public LoginQueue Login { get; } = new();

    public void Dispose()
    {
        Duty.Dispose();
        Login.Dispose();
    }
}
