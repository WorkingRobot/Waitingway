using Dalamud.Networking.Http;
using System;
using System.Diagnostics;
using System.Diagnostics.CodeAnalysis;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Waitingway.Api.Duty;
using Waitingway.Api.Login;
using Waitingway.Api.Models;
using Waitingway.Utils;

namespace Waitingway.Api;

public sealed class Api : IDisposable
{
    public LoginApi Login { get; }
    public DutyApi Duty { get; }

    public HttpClient Client { get; set; }
    private HappyEyeballsCallback HeCallback { get; }
    public JsonSerializerOptions JsonOptions { get; }

    private Task<VersionInfo>? ServerVersionTask { get; set; }
    public VersionInfo? ServerVersion => ServerVersionTask?.IsCompletedSuccessfully ?? false ? ServerVersionTask.Result : null;

    private const string Password = "🏳️‍⚧️";

    public Api()
    {
        HeCallback = new();
        JsonOptions = new JsonSerializerOptions()
        {
            PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower
        };
        JsonOptions.Converters.Add(new TimeSpanConverter());
        RefreshHttpConfiguration();
        Login = new(this);
        Duty = new(this);
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
        client.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("Waitingway", $"{Service.Version.VersionString}-{Service.Version.BuildConfiguration}"));
        return client;
    }

    /*
     * V2 API
     * ------
     * v1/queue_size => v2/queue/login/size
     * v1/recap => v2/queue/login/recap
     * v1/queue => v2/queue/login
     * v1/notifications => v2/queue/login/notifications
     */

    public const string EP_OAUTH_REDIRECT = "api/v1/oauth/redirect";
    public const string EP_VERSION = "api/v1/version";
    public const string EP_CONNECTIONS = "api/v1/connections";

    public const string EP_QUEUE_LOGIN_SIZE = "api/v2/queue/login/size";
    public const string EP_QUEUE_LOGIN_RECAP = "api/v2/queue/login/recap";
    public const string EP_QUEUE_LOGIN_GET = "api/v2/queue/login";
    public const string EP_QUEUE_LOGIN_NOTIFICATIONS = "api/v2/queue/login/notifications";

    public const string EP_QUEUE_DUTY_SIZE = "api/v2/queue/duty/roulette/size";
    public const string EP_QUEUE_DUTY_RECAP = "api/v2/queue/duty/recap";
    public const string EP_QUEUE_DUTY_GET = "api/v2/queue/duty/roulette";
    public const string EP_QUEUE_DUTY_NOTIFICATIONS = "api/v2/queue/duty/notifications";

    [MemberNotNull(nameof(Client))]
    public void RefreshHttpConfiguration()
    {
        var oldClient = Client;
        Client = CreateClient();
        oldClient?.Dispose();
        ServerVersionTask = Task.Run(GetVersionAsync);
        _ = ServerVersionTask.ContinueWith(t =>
        {
            if (t != ServerVersionTask)
                return;
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Waitingway server is unavailable", "Waitingway Server Unavailable");
        }, TaskContinuationOptions.OnlyOnFaulted);
        _ = ServerVersionTask.ContinueWith(t =>
        {
            if (t != ServerVersionTask)
                return;
            if (Service.Version.Version.Major != t.Result.SupportedVersionMajor || Service.Version.Version.Minor < t.Result.SupportedVersionMinor)
                Log.WarnNotify("Waitingway is outdated and may not work correctly. Please update for the latest features and bug fixes.", "Waitingway Server Version Mismatch");
        }, TaskContinuationOptions.OnlyOnRanToCompletion);

        Login?.ClearWorldQueueCache();
        Duty?.ClearRouletteQueueCache();
    }

    public async Task OpenOAuthInBrowserAsync()
    {
        var resp = await Client.GetAsync(EP_OAUTH_REDIRECT).ConfigureAwait(false);
        if (resp.StatusCode != HttpStatusCode.Found)
            throw new ApiException(EP_OAUTH_REDIRECT, $"Unexpected status code {resp.StatusCode}");
        var location = resp.Headers.Location ?? throw new ApiException(EP_OAUTH_REDIRECT, "No Location header");

        Process.Start(new ProcessStartInfo { FileName = location.AbsoluteUri, UseShellExecute = true });
    }

    public async Task<VersionInfo> GetVersionAsync()
    {
        var resp = (await Client.GetAsync(EP_VERSION).ConfigureAwait(false)).EnsureSuccessStatusCode();
        return await resp.Content.ReadFromJsonAsync<VersionInfo>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(EP_VERSION, "Json returned null");
    }

    #region Connections
    
    public async Task<Connection[]> GetConnectionsAsync()
    {
        var resp = (await Client.GetAsync(EP_CONNECTIONS).ConfigureAwait(false)).EnsureSuccessStatusCode();
        return await resp.Content.ReadFromJsonAsync<Connection[]>(JsonOptions).ConfigureAwait(false) ?? throw new ApiException(EP_CONNECTIONS, "Json returned null");
    }

    public async Task DeleteConnectionAsync(ulong connUserId)
    {
        var resp = (await Client.DeleteAsync($"{EP_CONNECTIONS}/{connUserId}").ConfigureAwait(false)).EnsureSuccessStatusCode();
        if (resp.StatusCode != HttpStatusCode.NoContent)
            throw new ApiException($"{EP_CONNECTIONS}/{connUserId}", $"Unexpected status code {resp.StatusCode}");
    }

    #endregion

    public void Dispose()
    {
        Client.Dispose();
        HeCallback.Dispose();
    }
}
