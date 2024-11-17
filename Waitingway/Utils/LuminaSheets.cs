using Dalamud.Utility;
using Lumina.Excel;
using Lumina.Excel.Sheets;
using SWorld = Lumina.Excel.Sheets.World;

namespace Waitingway.Utils;

public static class LuminaSheets
{
    private static readonly ExcelModule Module = Service.DataManager.GameData.Excel;

    public static readonly ExcelSheet<SWorld> World = Module.GetSheet<SWorld>()!;
    public static readonly ExcelSheet<WorldDCGroupType> WorldDCGroupType = Module.GetSheet<WorldDCGroupType>()!;
    public static readonly ExcelSheet<Error> Error = Module.GetSheet<Error>()!;
}
