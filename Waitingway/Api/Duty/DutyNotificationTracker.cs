using Lumina.Excel.Sheets;
using System;
using System.Diagnostics;
using System.Linq;
using System.Threading.Tasks;
using Waitingway.Api.Duty.Models;
using Waitingway.Api.Models;
using Waitingway.Utils;
using static Waitingway.Hooks.DutyQueue;

namespace Waitingway.Api.Duty;

public sealed class DutyNotificationTracker : IDisposable
{
    private Api Api { get; }

    private NotificationData? CurrentNotification { get; set; }
    private bool? SentRoulettePosition { get; set; }

    public DutyNotificationTracker()
    {
        Api = Service.Api;

        Service.DutyTracker.OnBeginQueue += OnBeginQueue;
        Service.DutyTracker.OnUpdateQueue += OnUpdateQueue;
        Service.DutyTracker.OnPopQueue += OnPopQueue;
        Service.DutyTracker.OnFinalizeQueue += OnFinalizeQueue;
    }

    public void Dispose()
    {
        if (CurrentNotification != null)
        {
            Log.WarnNotify("Currently in queue. Considering this queue unsuccessful.", "Unsuccessful Queue");
            DeleteNotificationFnf(new DeleteNotificationData { PositionStart = null, PositionEnd = null, Duration = 0, ResultingContent = null, ErrorMessage = null, ErrorCode = null }).Wait();
        }
    }

    private void OnBeginQueue()
    {
        SentRoulettePosition = null;
        if (CurrentNotification != null)
        {
            Log.ErrorNotify("Queue notification already exists, deleting", "Unexpected Notification");
            DeleteNotificationFnf(new DeleteNotificationData { PositionStart = null, PositionEnd = null, Duration = 0, ResultingContent = null, ErrorMessage = null, ErrorCode = null }).Wait();
        }
    }

    private void OnUpdateQueue()
    {
        var obj = Service.DutyTracker.CurrentRecap ?? throw new UnreachableException("No recap available");

        var update = obj.LastUpdate ?? throw new UnreachableException("No updates available");

        if (update is not WaitTimeUpdate waitUpdate)
            return;

        if (!obj.Party.HasValue && obj.QueuedRoulette.HasValue && update is RouletteUpdate rouletteUpdate && obj.Role is { } role)
        {
            if (obj.Updates.Count == 1)
            {
                if (!rouletteUpdate.IsIndeterminate)
                {
                    _ = SendQueueSizeFnf(new RouletteSize
                    {
                        WorldId = obj.WorldId,
                        RouletteId = obj.QueuedRoulette.Value,
                        EstimatedWaitTime = rouletteUpdate.WaitTimeMinutes,
                        Size = rouletteUpdate.RawPosition,
                        Role = role
                    });
                    SentRoulettePosition = true;
                }
                else
                {
                    _ = SendQueueSizeFnf(new RouletteSize
                    {
                        WorldId = obj.WorldId,
                        RouletteId = obj.QueuedRoulette.Value,
                        EstimatedWaitTime = rouletteUpdate.WaitTimeMinutes,
                        Size = null,
                        Role = role
                    });
                    SentRoulettePosition = false;
                }
            }
            else if (SentRoulettePosition == false && !rouletteUpdate.IsIndeterminate)
            {
                _ = SendQueueSizeFnf(new RouletteSize
                {
                    WorldId = obj.WorldId,
                    RouletteId = obj.QueuedRoulette.Value,
                    EstimatedWaitTime = null,
                    Size = rouletteUpdate.RawPosition,
                    Role = role
                });
                SentRoulettePosition = true;
            }
        }
        
        Task CreateNotificationFnf() =>
            this.CreateNotificationFnf(new CreateNotificationData
            {
                CharacterName = obj.CharacterName,
                HomeWorldId = (ushort)Service.ClientState.LocalPlayer!.HomeWorld.RowId,
                QueuedJob = obj.QueuedJob,
                QueuedRoulette = obj.QueuedRoulette,
                QueuedContent = obj.QueuedContent,
                Update = update,
                EstimatedTime = null,
            });

        if (obj.Updates.Count == 1)
        {
            if (Service.Configuration.DutyNotificationThreshold is { } threshold
                && threshold <= waitUpdate.WaitTimeMinutes)
                _ = CreateNotificationFnf();
        }
        else if (CurrentNotification == null && obj.Updates.Count > 1)
        {
            if (Service.Configuration.DutyNotificationThreshold is { } threshold
                && threshold <= ((WaitTimeUpdate)obj.Updates.First(u => u is WaitTimeUpdate)).WaitTimeMinutes)
                _ = CreateNotificationFnf();
        }
        else if (CurrentNotification != null)
        {
            _ = UpdateNotificationFnf(new QueueUpdateNotificationData
            {
                Update = update,
                EstimatedTime = null,
            });
        }
    }

    private void OnPopQueue()
    {
        if (CurrentNotification == null)
            return;

        var obj = Service.DutyTracker.CurrentRecap ?? throw new UnreachableException("No recap available");
        var pop = obj.LastPop ?? throw new UnreachableException("No queue pop available");

        _ = UpdateNotificationFnf(new PopUpdateNotificationData
        {
            Timestamp = pop.Timestamp,
            ResultingContent = pop.ResultingContent,
            InProgressBeginTimestamp = pop.InProgressBeginTimestamp
        });
    }

    private void OnFinalizeQueue()
    {
        var obj = Service.DutyTracker.CurrentRecap ?? throw new UnreachableException("No recap available");

        _ = CreateRecapFnf(obj);

        if (CurrentNotification == null)
            return;

        var positions = obj.Updates.Select(u => u as RouletteUpdate).Where(u => u != null && u.RawPosition is not (255 or 0));
        _ = DeleteNotificationFnf(new DeleteNotificationData
        {
            PositionStart = positions.FirstOrDefault()?.Position,
            PositionEnd = positions.LastOrDefault()?.Position,
            Duration = (uint)(obj.EndTime!.Value - obj.StartTime).TotalSeconds,
            ResultingContent = obj.LastPop?.ResultingContent,
            ErrorMessage = obj.WithdrawMessage is { } row ? LuminaSheets.LogMessage.GetRowOrDefault(row)?.Text.ExtractText() : null,
            ErrorCode = obj.WithdrawMessage
        });
    }

    private Task CreateRecapFnf(DutyQueueTracker.Recap recap)
    {
        var task = Api.Duty.CreateRecapAsync(recap);
        _ = task.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to publish queue recap", "Couldn't Publish Recap");
            else
                Log.Debug("Created recap");
        });
        return task;
    }

    private Task SendQueueSizeFnf(RouletteSize size)
    {
        var task = Api.Duty.SendRouletteSizeAsync(size);
        _ = task.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to send queue size", "Couldn't Send Queue Size");
            else
                Log.Debug("Sent queue size");
        });
        return task;
    }

    private Task CreateNotificationFnf(CreateNotificationData data)
    {
        if (CurrentNotification != null)
            throw new InvalidOperationException("Notifications cannot exist");

        var task = Api.Duty.CreateNotificationAsync(data);
        _ = task.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to create notification", "Couldn't Send Notification");
            else if (t.Result is { } notification)
            {
                CurrentNotification = notification;
                Log.Debug($"Created notification ({notification.Data} ; {notification.Nonce})");
            }
            else
                Log.WarnNotify("Your queue is too short. You won't get any notifications.", "Queue Notification Disabled");
        });
        return task;
    }

    private Task UpdateNotificationFnf(UpdateNotificationData data)
    {
        if (CurrentNotification == null)
            throw new InvalidOperationException("No notification to update");

        var task = Api.Duty.UpdateNotificationAsync(CurrentNotification, data);
        _ = task.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to update notification", "Couldn't Update Notification");
            else
                Log.Debug("Updated notification");
        });
        return task;
    }

    private Task DeleteNotificationFnf(DeleteNotificationData data)
    {
        if (CurrentNotification == null)
            throw new InvalidOperationException("No notification to delete");

        var notification = CurrentNotification;
        CurrentNotification = null;
        var task = Api.Duty.DeleteNotificationAsync(notification, data);
        _ = task.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to complete queue notification", "Couldn't Send Notification");
            else
                Log.Debug("Deleted unexpected notification");
        });
        return task;
    }
}
