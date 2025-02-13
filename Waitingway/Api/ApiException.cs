using System;

namespace Waitingway.Api;

public sealed class ApiException(string endpoint, string message) : Exception($"{message} ({endpoint})")
{
}
