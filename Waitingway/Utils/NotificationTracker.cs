using Dalamud.Utility;
using System;
using System.Diagnostics;
using System.Threading.Tasks;
using static Waitingway.Utils.Api;

namespace Waitingway.Utils;

public sealed class NotificationTracker : IDisposable
{
    private Api Api { get; }

    private NotificationData? CurrentNotification { get; set; }

    public NotificationTracker()
    {
        Api = Service.Api;

        Service.QueueTracker.OnBeginQueue += OnBeginQueue;
        Service.QueueTracker.OnUpdateQueue += OnUpdateQueue;
        Service.QueueTracker.OnCompleteQueue += OnCompleteQueue;
    }
    
    public void Dispose()
    {
        if (CurrentNotification != null)
        {
            Log.WarnNotify("Currently in queue. Considering this queue unsuccessful.", "Unsuccessful Queue");
            DeleteNotificationFnf(new DeleteNotificationData { Successful = false, QueueStartSize = 0, QueueEndSize = 0, Duration = 0, ErrorCode = null, ErrorMessage = null, IdentifyTimeout = null }).Wait();
        }
    }

    private void OnBeginQueue()
    {
        if (CurrentNotification != null)
        {
            Log.ErrorNotify("Queue notification already exists, deleting", "Unexpected Notification");
            _ = DeleteNotificationFnf(new DeleteNotificationData { Successful = false, QueueStartSize = 0, QueueEndSize = 0, Duration = 0, ErrorCode = null, ErrorMessage = null, IdentifyTimeout = null });
        }
    }

    private void OnUpdateQueue()
    {
        var obj = Service.QueueTracker.CurrentRecap ?? throw new UnreachableException("No recap available");

        var position = obj.CurrentPosition ?? throw new UnreachableException("No positions available");

        if (obj.Positions.Count == 1)
            _ = SendQueueSizeFnf(obj.WorldId, position.PositionNumber);

        if (obj.Positions.Count == 1)
        {
            if (Service.Configuration.NotificationThreshold <= position.PositionNumber)
            {
                _ = CreateNotificationFnf(new CreateNotificationData
                {
                    CharacterName = obj.CharacterName,
                    HomeWorldId = obj.HomeWorldId,
                    WorldId = obj.WorldId,
                    Position = (uint)position.PositionNumber,
                    UpdatedAt = position.Time,
                    EstimatedTime = obj.EstimateEndTime(position.Time)
                });
            }
        }
        else if (CurrentNotification == null && obj.Positions.Count > 1)
        {
            if (Service.Configuration.NotificationThreshold <= obj.Positions[0].PositionNumber)
            {
                _ = CreateNotificationFnf(new CreateNotificationData
                {
                    CharacterName = obj.CharacterName,
                    HomeWorldId = obj.HomeWorldId,
                    WorldId = obj.WorldId,
                    Position = (uint)position.PositionNumber,
                    UpdatedAt = position.Time,
                    EstimatedTime = obj.EstimateEndTime(position.Time)
                });
            }
        }
        else if (CurrentNotification != null)
        {
            _ = UpdateNotificationFnf(new UpdateNotificationData
            {
                Position = (uint)position.PositionNumber,
                UpdatedAt = position.Time,
                EstimatedTime = obj.EstimateEndTime(position.Time)
            });
        }
    }

    private void OnCompleteQueue()
    {
        var obj = Service.QueueTracker.CurrentRecap ?? throw new UnreachableException("No recap available");

        _ = CreateRecapFnf(obj);

        if (CurrentNotification != null)
            _ = DeleteNotificationFnf(
                new DeleteNotificationData {
                    Successful = obj.Successful,
                    QueueStartSize = (uint)obj.Positions[0].PositionNumber,
                    QueueEndSize = (uint)obj.Positions[^1].PositionNumber,
                    Duration = (uint)(obj.EndTime - obj.StartTime).TotalSeconds,
                    ErrorCode = obj.Error?.Code,
                    ErrorMessage = obj.Error?.ErrorRow is { } errorRow ? LuminaSheets.Error.GetRow(errorRow)?.Unknown0.ToDalamudString().TextValue : null,
                    IdentifyTimeout = !obj.IsIdentifyExpired ? obj.IdentifyTimeout : null
                });
    }

    private Task CreateRecapFnf(QueueTracker.Recap recap)
    {
        var task = Api.CreateRecapAsync(recap);
        _ = task.ContinueWith(t =>
        {
            if (t.Exception is { } e)
                Log.ErrorNotify(e, "Failed to publish queue recap", "Couldn't Publish Recap");
            else
                Log.Debug("Created recap");
        });
        return task;
    }

    private Task SendQueueSizeFnf(ushort worldId, int size)
    {
        var task = Api.SendQueueSizeAsync(worldId, size);
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
        
        var task = Api.CreateNotificationAsync(data);
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

        var task = Api.UpdateNotificationAsync(CurrentNotification, data);
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
        var task = Api.DeleteNotificationAsync(notification, data);
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
