using System;
using System.Collections.Generic;
using System.Linq;

namespace Waitingway.Utils;

public static class Estimator
{
    public static DateTime EstimateRate(IEnumerable<(DateTime Time, int Position)> history, DateTime now, float defaultPositionsPerMinute, Func<int, double> weightFunction)
    {
        // position / second
        List<double> rateAverage = [];

        history = history.TakeLast(15);

        var n = 0;
        foreach(var (dtime, dposition) in history
            .Skip(1)
            .Zip(history, (after, before) => (after.Time - before.Time, before.Position - after.Position)))
        {
            var rate = dposition / dtime.TotalNanoseconds;
            double nextRate;
            if (n == 0)
                nextRate = rate;
            else
                nextRate = double.Lerp(rate, rateAverage[n - 1], weightFunction(n));
            rateAverage.Add(nextRate);

            n++;
        }

        var lastEntry = history.Last();
        var lastRate = rateAverage.LastOrDefault();
        if (lastRate == 0)
            lastRate = defaultPositionsPerMinute / TimeSpan.FromMinutes(1).TotalNanoseconds;

        var timeOffset = now - lastEntry.Time;
        var retNanoseconds = (lastEntry.Position / lastRate) - (timeOffset.TotalNanoseconds * lastRate);
        return lastEntry.Time + new TimeSpan((long)Math.Round(retNanoseconds / 100));
    }

    public static double InverseWeight(int n) => n == 1 ? 0.5 : 1.0 / n;

    public static double ShiftedInverseWeight(int n) => 1.0 / (n + 1);

    public static double GeometricWeight(int n) => 0.5;

    public static double MinorGeometricWeight(int n) => 0.05;
}
