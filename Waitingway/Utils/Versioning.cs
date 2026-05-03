using System;
using System.Reflection;
using Dalamud.Interface.Textures;

namespace Waitingway.Utils;

public sealed class Versioning
{
    public Version Version { get; }
    public string VersionString { get; }
    public string Author { get; }
    public string BuildConfiguration { get; }
    public ISharedImmediateTexture Icon { get; }

    public Versioning()
    {
        var assembly = Assembly.GetExecutingAssembly();
        Version = assembly.GetName().Version!;
        VersionString = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()!.InformationalVersion.Split('+')[0];
        Author = assembly.GetCustomAttribute<AssemblyCompanyAttribute>()!.Company;
        BuildConfiguration = assembly.GetCustomAttribute<AssemblyConfigurationAttribute>()!.Configuration;
        Icon = IconManager.GetAssemblyTexture("icon.png");
    }
}
