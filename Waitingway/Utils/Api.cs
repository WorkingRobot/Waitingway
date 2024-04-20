using Dalamud.Networking.Http;
using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using System.Web;

namespace Waitingway.Utils;

public sealed class Api : IDisposable
{
    private HttpClient Client { get; set; }
    private HappyEyeballsCallback HeCallback { get; }
    public JsonSerializerOptions JsonOptions { get; }

    private const string Password = "🏳️‍⚧️";

    public Api()
    {
        HeCallback = new();
        JsonOptions = new JsonSerializerOptions()
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        };
        RefreshHttpConfiguration();
    }
    
    private HttpClient CreateClient()
    {
        var client = new HttpClient(new SocketsHttpHandler()
        {
            AutomaticDecompression = DecompressionMethods.All,
            ConnectCallback = HeCallback.ConnectCallback,
            AllowAutoRedirect = false
        });
        client.BaseAddress = Service.Configuration.ServerUri;
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Basic", Convert.ToBase64String(Encoding.UTF8.GetBytes($"{Service.Configuration.ClientId}:{Password}")));
        client.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("Waitingway", "1.0"));
        return client;
    }

    [MemberNotNull(nameof(Client))]
    public void RefreshHttpConfiguration()
    {
        var oldClient = Client;
        Client = CreateClient();
        oldClient?.Dispose();
    }

    public async Task OpenConnectionLinkInBrowserAsync()
    {
        var resp = (await Client.GetAsync("api/v1/oauth/redirect").ConfigureAwait(false));
        if (resp.StatusCode != HttpStatusCode.Found)
            throw new ApiException("oauth/redirect", $"Unexpected status code {resp.StatusCode}");
        var location = resp.Headers.Location ?? throw new ApiException("oauth/redirect", "No Location header");

        Process.Start(new ProcessStartInfo { FileName = location.AbsoluteUri, UseShellExecute = true });
    }

    public async Task<Connection[]> GetConnectionsAsync()
    {
        var resp = (await Client.GetAsync("api/v1/connections").ConfigureAwait(false)).EnsureSuccessStatusCode();
        return (await resp.Content.ReadFromJsonAsync<Connection[]>(JsonOptions).ConfigureAwait(false)) ?? throw new ApiException("api/v1/connections", "Json returned null");
    }

    public async Task DeleteConnectionAsync(ulong connUserId)
    {
        var resp = (await Client.DeleteAsync($"api/v1/connections/{connUserId}").ConfigureAwait(false)).EnsureSuccessStatusCode();
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException($"api/v1/connections/{connUserId}", $"Unexpected status code {resp.StatusCode}");
    }

    public async Task CreateRecapAsync(QueueTracker.Recap recap)
    {
        var resp = (await Client.PostAsJsonAsync("api/v1/recap", recap, JsonOptions).ConfigureAwait(false));//.EnsureSuccessStatusCode();
        if (!resp.IsSuccessStatusCode)
            Log.Warn($"{await resp.Content.ReadAsStringAsync().ConfigureAwait(false)}");
        if (resp.StatusCode != HttpStatusCode.Created)
            throw new ApiException("api/v1/recap", $"Unexpected status code {resp.StatusCode}");
    }

    public async Task<QueueEstimate[]> GetAllQueuesAsync()
    {
        var resp = (await Client.GetAsync("api/v1/queue").ConfigureAwait(false)).EnsureSuccessStatusCode();
        return (await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false)) ?? throw new ApiException("api/v1/queues", "Json returned null");
    }

    public async Task<QueueEstimate?> GetWorldQueueAsync(ushort worldId)
    {
        var uri = new UriBuilder("api/v1/queue");
        var qs = HttpUtility.ParseQueryString(string.Empty);
        qs["world_id"] = worldId.ToString();
        uri.Query = qs.ToString();

        var resp = (await Client.GetAsync(uri.Uri).ConfigureAwait(false)).EnsureSuccessStatusCode();
        return ((await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false)) ?? throw new ApiException(uri.ToString(), "Json returned null")).FirstOrDefault();
    }

    public async Task<QueueEstimate[]> GetDatacenterQueuesAsync(ushort datacenterId)
    {
        var uri = new UriBuilder("api/v1/queue");
        var qs = HttpUtility.ParseQueryString(string.Empty);
        qs["datacenter_id"] = datacenterId.ToString();
        uri.Query = qs.ToString();

        var resp = (await Client.GetAsync(uri.Uri).ConfigureAwait(false)).EnsureSuccessStatusCode();
        return (await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false)) ?? throw new ApiException(uri.ToString(), "Json returned null");
    }

    public async Task<QueueEstimate[]> GetRegionQueuesAsync(ushort regionId)
    {
        var uri = new UriBuilder("api/v1/queue");
        var qs = HttpUtility.ParseQueryString(string.Empty);
        qs["region_id"] = regionId.ToString();
        uri.Query = qs.ToString();

        var resp = (await Client.GetAsync(uri.Uri).ConfigureAwait(false)).EnsureSuccessStatusCode();
        return (await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false)) ?? throw new ApiException(uri.ToString(), "Json returned null");
    }

    public async Task<NotificationData?> CreateNotificationAsync(CreateNotificationData data)
    {
        var resp = (await Client.PostAsJsonAsync("api/v1/notifications", data, JsonOptions).ConfigureAwait(false)).EnsureSuccessStatusCode();
        if (resp.StatusCode == HttpStatusCode.NoContent)
            return null;
        if (resp.StatusCode == HttpStatusCode.Created)
        {
            if (!resp.Headers.TryGet("X-Instance-Nonce", out var instNonce))
                throw new ApiException("api/v1/notifications", "No nonce header");
            if (!resp.Headers.TryGet("X-Instance-Data", out var instData))
                throw new ApiException("api/v1/notifications", "No data header");

            return new NotificationData { Nonce = instNonce, Data = instData };
        }
        throw new ApiException("api/v1/notifications", $"Unexpected status code {resp.StatusCode}");
    }

    public async Task UpdateNotificationAsync(NotificationData notificationData, UpdateNotificationData data)
    {
        HttpResponseMessage resp;
        using (var message = new HttpRequestMessage(HttpMethod.Patch, "api/v1/notifications"))
        {
            message.Headers.Add("X-Instance-Nonce", notificationData.Nonce);
            message.Headers.Add("X-Instance-Data", notificationData.Data);
            message.Content = JsonContent.Create(data, options: JsonOptions);

            resp = (await Client.SendAsync(message).ConfigureAwait(false)).EnsureSuccessStatusCode();
        }
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException("api/v1/notifications", $"Unexpected status code {resp.StatusCode}");
    }

    public async Task DeleteNotificationAsync(NotificationData notificationData, DeleteNotificationData data)
    {
        HttpResponseMessage resp;
        using (var message = new HttpRequestMessage(HttpMethod.Delete, "api/v1/notifications"))
        {
            message.Headers.Add("X-Instance-Nonce", notificationData.Nonce);
            message.Headers.Add("X-Instance-Data", notificationData.Data);
            message.Content = JsonContent.Create(data, options: JsonOptions);

            resp = (await Client.SendAsync(message).ConfigureAwait(false));//.EnsureSuccessStatusCode();
        }
        if (!resp.IsSuccessStatusCode)
            Log.Warn($"{await resp.Content.ReadAsStringAsync().ConfigureAwait(false)}");

        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException("api/v1/notifications", $"Unexpected status code {resp.StatusCode}");
    }

    public void Dispose()
    {
        Client.Dispose();
        HeCallback.Dispose();
    }

    public sealed record Connection
    {
        public required DateTime CreatedAt { get; init; }

        public required ulong ConnUserId { get; init; }
        public required string Username { get; init; }
        public required string DisplayName { get; init; }
    }

    public sealed record QueueEstimate
    {
        public required ushort WorldId { get; init; }
        public required ushort DatacenterId { get; init; }
        public required ushort RegionId { get; init; }

        public required TimeSpan EstimatedQueueTime { get; init; }
    }

    public sealed record CreateNotificationData
    {
        public required string CharacterName { get; init; }
        public required ushort HomeWorldId { get; init; }
        public required ushort WorldId { get; init; }
        public required uint Position { get; init; }
        public required DateTime UpdatedAt { get; init; }
        public required DateTime EstimatedTime { get; init; }
    }

    public sealed record UpdateNotificationData
    {
        public required uint Position { get; init; }
        public required DateTime UpdatedAt { get; init; }
        public required DateTime EstimatedTime { get; init; }
    }

    public sealed record DeleteNotificationData
    {
        public required bool Successful { get; init; }
        public required uint QueueStartSize { get; init; }
        public required uint QueueEndSize { get; init; }
        public required uint Duration { get; init; }
    }

    public sealed record NotificationData
    {
        public required string Nonce { get; init; }
        public required string Data { get; init; }
    }
}

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

public sealed class ApiException(string endpoint, string message) : Exception($"{message} ({endpoint})")
{
}
