using Dalamud.Interface.Textures;
using System.Reflection;

namespace Waitingway.Utils;

public static class IconManager
{
    public static ISharedImmediateTexture GetAssemblyTexture(string filename) =>
        Service.TextureProvider.GetFromManifestResource(Assembly.GetExecutingAssembly(), $"Waitingway.{filename}");
}
