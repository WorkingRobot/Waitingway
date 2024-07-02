using Dalamud.Interface.Textures.TextureWraps;
using Dalamud.Interface.Textures;
using System;
using System.Collections.Generic;
using System.Reflection;
using System.Numerics;
using System.Threading.Tasks;
using System.Threading;
using Dalamud.Utility;

namespace Waitingway.Utils;

public interface ITextureIcon
{
    ISharedImmediateTexture Source { get; }

    Vector2? Dimensions { get; }

    float? AspectRatio => Dimensions is { } d ? d.X / d.Y : null;

    nint ImGuiHandle { get; }

    IDalamudTextureWrap GetWrap();
}

public interface ILoadedTextureIcon : ITextureIcon, IDisposable { }

public sealed class IconManager : IDisposable
{
    private sealed class LoadedIcon : ILoadedTextureIcon
    {
        // 10: DXGI_FORMAT_R16G16B16A16_FLOAT
        public static IDalamudTextureWrap EmptyTexture { get; } = Service.TextureProvider.CreateEmpty(new(4, 4, 10), false, false);

        public ISharedImmediateTexture Source { get; }

        public Vector2? Dimensions => GetWrap()?.Size;

        public nint ImGuiHandle => GetWrapOrEmpty().ImGuiHandle;

        private Task<IDalamudTextureWrap> TextureWrapTask { get; }
        private CancellationTokenSource DisposeToken { get; }

        public LoadedIcon(ISharedImmediateTexture source)
        {
            Source = source;
            DisposeToken = new();
            TextureWrapTask = source.RentAsync(DisposeToken.Token);
        }

        public IDalamudTextureWrap GetWrap() =>
            TextureWrapTask.GetAwaiter().GetResult();

        public IDalamudTextureWrap? TryGetWrap()
        {
            if (TextureWrapTask.IsCompletedSuccessfully)
                return TextureWrapTask.Result;
            return null;
        }

        public IDalamudTextureWrap GetWrapOrEmpty() => TryGetWrap() ?? EmptyTexture;

        public void Dispose()
        {
            DisposeToken.Cancel();
            TextureWrapTask.ToContentDisposedTask(true).Wait();
        }
    }

    // TODO: Unload when unused, but with a custom timer?
    private sealed class CachedIcon : ITextureIcon
    {
        private LoadedIcon Base { get; }

        public ISharedImmediateTexture Source => Base.Source;

        public Vector2? Dimensions => Base.Dimensions;

        public nint ImGuiHandle => Base.ImGuiHandle;

        public CachedIcon(ISharedImmediateTexture source)
        {
            Base = new(source);
        }

        public IDalamudTextureWrap GetWrap()
        {
            return Base.GetWrap();
        }

        public void Release()
        {
            Base.Dispose();
        }
    }

    private Dictionary<string, CachedIcon> AssemblyTextureCache { get; } = [];

    private static ISharedImmediateTexture GetAssemblyTextureInternal(string filename) =>
        Service.TextureProvider.GetFromManifestResource(Assembly.GetExecutingAssembly(), $"Waitingway.{filename}");

    public static ILoadedTextureIcon GetAssemblyTexture(string filename) =>
        new LoadedIcon(GetAssemblyTextureInternal(filename));

    public ITextureIcon GetAssemblyTextureCached(string filename)
    {
        if (AssemblyTextureCache.TryGetValue(filename, out var texture))
            return texture;
        return AssemblyTextureCache[filename] = new(GetAssemblyTextureInternal(filename));
    }

    public void Dispose()
    {
        foreach (var value in AssemblyTextureCache.Values)
            value.Release();
    }
}
