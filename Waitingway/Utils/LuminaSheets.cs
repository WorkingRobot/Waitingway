using Dalamud.Utility;
using ExdSheets;
using ExdSheets.Sheets;
using SWorld = ExdSheets.Sheets.World;

namespace Waitingway.Utils;

public static class LuminaSheets
{
    private static readonly Module Module = new(Service.DataManager.GameData, Service.DataManager.Language.ToLumina());

    public static readonly Sheet<SWorld> World = Module.GetSheet<SWorld>()!;
    public static readonly Sheet<WorldDCGroupType> WorldDCGroupType = Module.GetSheet<WorldDCGroupType>()!;
    public static readonly Sheet<Error> Error = Module.GetSheet<Error>()!;
}
