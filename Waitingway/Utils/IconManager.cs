using Dalamud.Interface.Internal;
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;

namespace Waitingway.Utils;

public sealed class IconManager : IDisposable
{
    private readonly Dictionary<string, IDalamudTextureWrap> assemblyCache = [];

    public IDalamudTextureWrap GetAssemblyTexture(string filename)
    {
        if (!assemblyCache.TryGetValue(filename, out var ret))
            assemblyCache.Add(filename, ret = GetAssemblyTextureInternal(filename));
        return ret;
    }

    private static IDalamudTextureWrap GetAssemblyTextureInternal(string filename)
    {
        var assembly = Assembly.GetExecutingAssembly();
        byte[] iconData;
        using (var stream = assembly.GetManifestResourceStream($"Waitingway.{filename}") ?? throw new InvalidDataException($"Could not load resource {filename}"))
        {
            iconData = new byte[stream.Length];
            _ = stream.Read(iconData);
        }
        return Service.PluginInterface.UiBuilder.LoadImage(iconData);
    }

    public void Dispose()
    {
        foreach (var image in assemblyCache.Values)
            image.Dispose();
        assemblyCache.Clear();
    }
}
