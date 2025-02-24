using Dalamud.Interface.ImGuiNotification;
using System;

namespace Waitingway.Utils;

public static class Log
{
    public static void Debug(string line) => Service.PluginLog.Debug(line);

    public static void Warn(string line) => Service.PluginLog.Warning(line);

    public static void WarnNotify(string line, string? title = null)
    {
        Service.PluginLog.Warning(line);

        DisplayNotification(new Notification
        {
            Type = NotificationType.Warning,
            Title = title,
            MinimizedText = title,
            Content = line,
            Minimized = false
        });
    }

    public static void Error(string line) => Service.PluginLog.Error(line);
    public static void Error(Exception e, string line)
    {
        e = FlattenException(e);

        Service.PluginLog.Error(e, line);
    }

    public static void ErrorNotify(string line, string? title = null)
    {
        Service.PluginLog.Error(line);

        DisplayNotification(new Notification
        {
            Type = NotificationType.Error,
            Title = title,
            MinimizedText = title,
            Content = line,
            Minimized = false
        });
    }

    public static void ErrorNotify(Exception e, string line, string? title = null)
    {
        e = FlattenException(e);

        Service.PluginLog.Error(e, line);

        DisplayNotification(new Notification
        {
            Type = NotificationType.Error,
            Title = title,
            Content = $"{line}\n{e.Message}",
            MinimizedText = title,
            Minimized = false
        });
    }

    public static IActiveNotification Notify(Notification n) =>
        DisplayNotification(n);

    private static Exception FlattenException(Exception e)
    {
        if (e is AggregateException { } aggExc)
            return aggExc.Flatten().InnerExceptions[0];
        return e;
    }

    private static IActiveNotification DisplayNotification(Notification n)
    {
        return Service.NotificationManager.AddNotification(n);
    }

    public static string GetTimeSpanFormat(TimeSpan span)
    {
        var neg = span.Ticks < 0 ? @"\-" : string.Empty;
        var day = Math.Abs(span.TotalDays) >= 1 ? @"d\d\ " : string.Empty;
        var hour = Math.Abs(span.TotalHours) >= 1 ? @"hh\:" : string.Empty;
        return @$"{neg}{day}{hour}mm\:ss";
    }
}
