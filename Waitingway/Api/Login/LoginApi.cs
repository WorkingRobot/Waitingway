using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using System.Threading.Tasks;
using System.Web;
using Waitingway.Api.Login.Models;
using Waitingway.Api.Models;
using Waitingway.Utils;

namespace Waitingway.Api.Login;

public sealed class LoginApi(Api api)
{
    private Api Api { get; } = api;

    private HttpClient Client => Api.Client;
    private JsonSerializerOptions JsonOptions => Api.JsonOptions;

    private Dictionary<ushort, Task<QueueEstimate?>> CachedQueueEstimates { get; } = [];

    public const string EP_QUEUE_LOGIN_SIZE = Api.EP_QUEUE_LOGIN_SIZE;
    public const string EP_QUEUE_LOGIN_RECAP = Api.EP_QUEUE_LOGIN_RECAP;
    public const string EP_QUEUE_LOGIN_GET = Api.EP_QUEUE_LOGIN_GET;
    public const string EP_QUEUE_LOGIN_NOTIFICATIONS = Api.EP_QUEUE_LOGIN_NOTIFICATIONS;

    public async Task SendQueueSizeAsync(ushort worldId, int size)
    {
        var resp = await Client.PostAsJsonAsync(EP_QUEUE_LOGIN_SIZE, new QueueSize { WorldId = worldId, Size = size }, JsonOptions).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.OK)
            throw new ApiException(EP_QUEUE_LOGIN_SIZE, $"Unexpected status code {resp.StatusCode}");
    }

    public async Task CreateRecapAsync(LoginQueueTracker.Recap recap)
    {
        var resp = await Client.PostAsJsonAsync(EP_QUEUE_LOGIN_RECAP, recap, JsonOptions).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.Created)
            throw new ApiException(EP_QUEUE_LOGIN_RECAP, $"Unexpected status code {resp.StatusCode}");
    }

    #region Queue Estimates

    public async Task<QueueEstimate[]> GetAllQueuesAsync()
    {
        var resp = await Client.GetAsync(EP_QUEUE_LOGIN_GET).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        return await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(EP_QUEUE_LOGIN_GET, "Json returned null");
    }

    public async Task<QueueEstimate[]> GetWorldQueuesAsync(IEnumerable<ushort> worldIds)
    {
        var uri = new UriBuilder(Client.BaseAddress + EP_QUEUE_LOGIN_GET);
        var qs = HttpUtility.ParseQueryString(string.Empty);
        foreach (var worldId in worldIds)
            qs.Add("world_id", worldId.ToString());
        uri.Query = qs.ToString();

        var resp = await Client.GetAsync(uri.Uri).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        return await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(uri.ToString(), "Json returned null");
    }

    public async Task<QueueEstimate[]> GetDatacenterQueuesAsync(IEnumerable<ushort> datacenterIds)
    {
        var uri = new UriBuilder(Client.BaseAddress + EP_QUEUE_LOGIN_GET);
        var qs = HttpUtility.ParseQueryString(string.Empty);
        foreach (var datacenterId in datacenterIds)
            qs.Add("datacenter_id", datacenterId.ToString());
        uri.Query = qs.ToString();

        var resp = await Client.GetAsync(uri.Uri).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        return await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(uri.ToString(), "Json returned null");
    }

    public async Task<QueueEstimate[]> GetRegionQueuesAsync(IEnumerable<ushort> regionIds)
    {
        var uri = new UriBuilder(Client.BaseAddress + EP_QUEUE_LOGIN_GET);
        var qs = HttpUtility.ParseQueryString(string.Empty);
        foreach (var regionId in regionIds)
            qs.Add("region_id", regionId.ToString());
        uri.Query = qs.ToString();

        var resp = await Client.GetAsync(uri.Uri).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        return await resp.Content.ReadFromJsonAsync<QueueEstimate[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(uri.ToString(), "Json returned null");
    }

    #endregion

    #region Notifications

    public async Task<NotificationData?> CreateNotificationAsync(CreateNotificationData data)
    {
        var resp = await Client.PostAsJsonAsync(EP_QUEUE_LOGIN_NOTIFICATIONS, data, JsonOptions).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode == HttpStatusCode.NoContent)
            return null;
        if (resp.StatusCode == HttpStatusCode.Created)
        {
            if (!resp.Headers.TryGet("X-Instance-Nonce", out var instNonce))
                throw new ApiException(EP_QUEUE_LOGIN_NOTIFICATIONS, "No nonce header");
            if (!resp.Headers.TryGet("X-Instance-Data", out var instData))
                throw new ApiException(EP_QUEUE_LOGIN_NOTIFICATIONS, "No data header");

            return new NotificationData { Nonce = instNonce, Data = instData };
        }
        throw new ApiException(EP_QUEUE_LOGIN_NOTIFICATIONS, $"Unexpected status code {resp.StatusCode}");
    }

    public async Task UpdateNotificationAsync(NotificationData notificationData, UpdateNotificationData data)
    {
        HttpResponseMessage resp;
        using (var message = new HttpRequestMessage(HttpMethod.Patch, EP_QUEUE_LOGIN_NOTIFICATIONS))
        {
            message.Headers.Add("X-Instance-Nonce", notificationData.Nonce);
            message.Headers.Add("X-Instance-Data", notificationData.Data);
            message.Content = JsonContent.Create(data, options: JsonOptions);

            resp = await Client.SendAsync(message).ConfigureAwait(false);
        }
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException(EP_QUEUE_LOGIN_NOTIFICATIONS, $"Unexpected status code {resp.StatusCode}");
    }

    public async Task DeleteNotificationAsync(NotificationData notificationData, DeleteNotificationData data)
    {
        HttpResponseMessage resp;
        using (var message = new HttpRequestMessage(HttpMethod.Delete, EP_QUEUE_LOGIN_NOTIFICATIONS))
        {
            message.Headers.Add("X-Instance-Nonce", notificationData.Nonce);
            message.Headers.Add("X-Instance-Data", notificationData.Data);
            message.Content = JsonContent.Create(data, options: JsonOptions);

            resp = await Client.SendAsync(message).ConfigureAwait(false);
        }
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException(EP_QUEUE_LOGIN_NOTIFICATIONS, $"Unexpected status code {resp.StatusCode}");
    }

    #endregion

    #region Cache

    public void ClearWorldQueueCache() =>
        CachedQueueEstimates.Clear();

    public CachedEstimate<QueueEstimate>[] GetWorldQueuesCached(params ushort[] worldIds)
    {
        var estimates = new Task<QueueEstimate?>[worldIds.Length];
        List<ushort> estimatesToRetrieve = [];

        for (var i = 0; i < worldIds.Length; ++i)
        {
            if (CachedQueueEstimates.TryGetValue(worldIds[i], out var estimate))
                estimates[i] = estimate;
            else
                estimatesToRetrieve.Add(worldIds[i]);
        }
        if (estimatesToRetrieve.Count > 0)
        {
            Log.Debug($"Getting world queues {string.Join(", ", worldIds)}");
            var ret = GetWorldQueuesAsync(estimatesToRetrieve);
            _ = ret.ContinueWith(t =>
            {
                if (t.Exception is { } e)
                    Log.ErrorNotify(e, "Failed to get queue estimates", "Couldn't Get Queue Info");
            }, TaskContinuationOptions.OnlyOnFaulted);
            for (var i = 0; i < worldIds.Length; ++i)
            {
                if (estimates[i] == null)
                {
                    var id = worldIds[i];
                    estimates[i] = CachedQueueEstimates[id] = ret.ContinueWith(t =>
                    {
                        if (t.Exception is { } e)
                            throw e;
                        return t.Result.FirstOrDefault(q => q.WorldId == id);
                    });
                }
            }
        }

        return estimates.Select(t =>
        {
            CacheState state;
            QueueEstimate? estimate = null;
            if (!t.IsCompleted)
                state = CacheState.InProgress;
            else if (t.IsFaulted)
                state = CacheState.Failed;
            else if (t.Result == null)
                state = CacheState.NotFound;
            else
            {
                state = CacheState.Found;
                estimate = t.Result;
            }
            return new CachedEstimate<QueueEstimate>
            {
                State = state,
                Estimate = estimate
            };
        }).ToArray();
    }

    #endregion
}
