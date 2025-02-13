using System.Diagnostics.CodeAnalysis;
using System.Linq;
using System.Net.Http.Headers;

namespace Waitingway.Api;

internal static class HttpExtensions
{
    public static bool TryGet(this HttpHeaders me, string key, [NotNullWhen(true)] out string? value)
    {
        if (me.TryGetValues(key, out var values))
        {
            value = values.FirstOrDefault();
            return value != null;
        }
        value = null;
        return false;
    }
}
