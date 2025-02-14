using System.Diagnostics.CodeAnalysis;
using System.Linq;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Threading.Tasks;
using Waitingway.Utils;

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

    public static async Task<HttpResponseMessage> EnsureSuccess(this HttpResponseMessage message)
    {
        if (!message.IsSuccessStatusCode)
        {
            Log.Error($"Received unsuccessful status code from {message.RequestMessage?.RequestUri}: {message.StatusCode}");
            if (message.RequestMessage?.Content is { } content)
                Log.Error($"Sent: {await content.ReadAsStringAsync().ConfigureAwait(false)}");
            Log.Error($"Returned: {await message.Content.ReadAsStringAsync().ConfigureAwait(false)}");
            message.EnsureSuccessStatusCode();
        }
        return message;
    }
}
