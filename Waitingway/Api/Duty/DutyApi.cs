using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using System.Threading.Tasks;
using System.Web;
using Waitingway.Api.Duty.Models;
using Waitingway.Api.Models;
using Waitingway.Utils;
using static Waitingway.Hooks.DutyQueue;

namespace Waitingway.Api.Duty;

public sealed class DutyApi(Api api)
{
    private Api Api { get; } = api;

    private HttpClient Client => Api.Client;
    private JsonSerializerOptions JsonOptions => Api.JsonOptions;

    private Dictionary<(ushort DatacenterId, QueueLanguage Language), Dictionary<byte, Task<Dictionary<RouletteRole, RouletteEstimate>>>> CachedRouletteEstimates { get; } = [];

    public const string EP_QUEUE_DUTY_SIZE = Api.EP_QUEUE_DUTY_SIZE;
    public const string EP_QUEUE_DUTY_RECAP = Api.EP_QUEUE_DUTY_RECAP;
    public const string EP_QUEUE_DUTY_GET = Api.EP_QUEUE_DUTY_GET;
    public const string EP_QUEUE_DUTY_NOTIFICATIONS = Api.EP_QUEUE_DUTY_NOTIFICATIONS;
    
    public async Task SendRouletteSizeAsync(RouletteSize size)
    {
        var resp = await Client.PostAsJsonAsync(EP_QUEUE_DUTY_SIZE, size, JsonOptions).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.OK)
            throw new ApiException(EP_QUEUE_DUTY_SIZE, $"Unexpected status code {resp.StatusCode}");
    }

    public async Task CreateRecapAsync(DutyQueueTracker.Recap recap)
    {
        var resp = await Client.PostAsJsonAsync(EP_QUEUE_DUTY_RECAP, recap, JsonOptions).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.Created)
            throw new ApiException(EP_QUEUE_DUTY_RECAP, $"Unexpected status code {resp.StatusCode}");
    }

    #region Roulette Estimates

    public async Task<RouletteEstimate[]> GetAllRouletteQueuesAsync()
    {
        var resp = await Client.GetAsync(EP_QUEUE_DUTY_GET).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        return await resp.Content.ReadFromJsonAsync<RouletteEstimate[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(EP_QUEUE_DUTY_GET, "Json returned null");
    }

    public async Task<RouletteEstimate[]> GetDatacenterRouletteQueuesAsync(ushort datacenterId, QueueLanguage languages, params IEnumerable<byte> rouletteIds)
    {
        var uri = new UriBuilder($"{Client.BaseAddress}{EP_QUEUE_DUTY_GET}/{datacenterId}");
        var qs = HttpUtility.ParseQueryString(string.Empty);
        qs.Add("lang", ((byte)languages).ToString());
        foreach (var rouletteId in rouletteIds)
            qs.Add("roulette_id", rouletteId.ToString());
        uri.Query = qs.ToString();

        var resp = await Client.GetAsync(uri.Uri).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        return await resp.Content.ReadFromJsonAsync<RouletteEstimate[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(uri.ToString(), "Json returned null");
    }

    #endregion

    #region Notifications

    public async Task<NotificationData?> CreateNotificationAsync(CreateNotificationData data)
    {
        var resp = await Client.PostAsJsonAsync(EP_QUEUE_DUTY_NOTIFICATIONS, data, JsonOptions).ConfigureAwait(false);
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode == HttpStatusCode.NoContent)
            return null;
        if (resp.StatusCode == HttpStatusCode.Created)
        {
            if (!resp.Headers.TryGet("X-Instance-Nonce", out var instNonce))
                throw new ApiException(EP_QUEUE_DUTY_NOTIFICATIONS, "No nonce header");
            if (!resp.Headers.TryGet("X-Instance-Data", out var instData))
                throw new ApiException(EP_QUEUE_DUTY_NOTIFICATIONS, "No data header");

            return new NotificationData { Nonce = instNonce, Data = instData };
        }
        throw new ApiException(EP_QUEUE_DUTY_NOTIFICATIONS, $"Unexpected status code {resp.StatusCode}");
    }

    public async Task UpdateNotificationAsync(NotificationData notificationData, UpdateNotificationData data)
    {
        HttpResponseMessage resp;
        using (var message = new HttpRequestMessage(HttpMethod.Patch, EP_QUEUE_DUTY_NOTIFICATIONS))
        {
            message.Headers.Add("X-Instance-Nonce", notificationData.Nonce);
            message.Headers.Add("X-Instance-Data", notificationData.Data);
            message.Content = JsonContent.Create(data, options: JsonOptions);

            resp = await Client.SendAsync(message).ConfigureAwait(false);
        }
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException(EP_QUEUE_DUTY_NOTIFICATIONS, $"Unexpected status code {resp.StatusCode}");
    }

    public async Task DeleteNotificationAsync(NotificationData notificationData, DeleteNotificationData data)
    {
        HttpResponseMessage resp;
        using (var message = new HttpRequestMessage(HttpMethod.Delete, EP_QUEUE_DUTY_NOTIFICATIONS))
        {
            message.Headers.Add("X-Instance-Nonce", notificationData.Nonce);
            message.Headers.Add("X-Instance-Data", notificationData.Data);
            message.Content = JsonContent.Create(data, options: JsonOptions);

            resp = await Client.SendAsync(message).ConfigureAwait(false);
        }
        await resp.EnsureSuccess().ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException(EP_QUEUE_DUTY_NOTIFICATIONS, $"Unexpected status code {resp.StatusCode}");
    }

    #endregion

    #region Cache

    public void ClearRouletteQueueCache() =>
        CachedRouletteEstimates.Clear();

    public CachedEstimate<Dictionary<RouletteRole, RouletteEstimate>>[] GetRouletteQueuesCached(ushort datacenterId, QueueLanguage languages, params byte[] rouletteIds)
    {
        var estimates = new Task<Dictionary<RouletteRole, RouletteEstimate>>[rouletteIds.Length];
        List<byte> estimatesToRetrieve = [];

        if (!CachedRouletteEstimates.TryGetValue((datacenterId, languages), out var dcCache))
            CachedRouletteEstimates[(datacenterId, languages)] = dcCache = [];
        for (var i = 0; i < rouletteIds.Length; ++i)
        {
            if (dcCache.TryGetValue(rouletteIds[i], out var estimate))
                estimates[i] = estimate;
            else
                estimatesToRetrieve.Add(rouletteIds[i]);
        }
        if (estimatesToRetrieve.Count > 0)
        {
            Log.Debug($"Getting roulette queues {string.Join(", ", rouletteIds)}");
            var ret = GetDatacenterRouletteQueuesAsync(datacenterId, languages, estimatesToRetrieve);
            _ = ret.ContinueWith(t =>
            {
                if (t.Exception is { } e)
                    Log.ErrorNotify(e, "Failed to get queue estimates", "Couldn't Get Queue Info");
            }, TaskContinuationOptions.OnlyOnFaulted);
            for (var i = 0; i < rouletteIds.Length; ++i)
            {
                if (estimates[i] == null)
                {
                    var id = rouletteIds[i];
                    estimates[i] = dcCache[id] = ret.ContinueWith(t =>
                    {
                        if (t.Exception is { } e)
                            throw e;
                        return t.Result.Where(q => q.RouletteId == id).ToDictionary(q => q.Role);
                    });
                }
            }
        }

        return estimates.Select(t =>
        {
            CacheState state;
            Dictionary<RouletteRole, RouletteEstimate>? estimate = null;
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
            return new CachedEstimate<Dictionary<RouletteRole, RouletteEstimate>>()
            {
                State = state,
                Estimate = estimate
            };
        }).ToArray();
    }

    #endregion
}
