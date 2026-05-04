using System;

namespace Waitingway.Hooks;

public sealed class Hooks : IDisposable
{
    public DutyQueue Duty { get; } = new();
    public LoginQueue Login { get; } = new();

    public void Dispose()
    {
        Duty.Dispose();
        Login.Dispose();
    }
}
