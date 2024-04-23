using Dalamud.Interface.Internal;
using System;
using System.Reflection;

namespace Waitingway.Utils;

public sealed class Versioning
{
    public Version Version { get; }
    public string VersionString { get; }
    public string Author { get; }
    public string BuildConfiguration { get; }
    public IDalamudTextureWrap Icon { get; }

    public Versioning()
    {
        var assembly = Assembly.GetExecutingAssembly();
        Version = assembly.GetName().Version!;
        VersionString = assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()!.InformationalVersion.Split('+')[0];
        Author = assembly.GetCustomAttribute<AssemblyCompanyAttribute>()!.Company;
        BuildConfiguration = assembly.GetCustomAttribute<AssemblyConfigurationAttribute>()!.Configuration;
        Icon = Service.IconManager.GetAssemblyTexture("icon.png");
    }
}
