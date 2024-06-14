using ExdSheets;
using Lumina.Excel;
using SWorld = ExdSheets.World;

namespace Waitingway.Utils;

public static class LuminaSheets
{
    public static readonly ExcelSheet<SWorld> World = Service.DataManager.GetExcelSheet<SWorld>()!;
    public static readonly ExcelSheet<WorldDCGroupType> WorldDCGroupType = Service.DataManager.GetExcelSheet<WorldDCGroupType>()!;
    public static readonly ExcelSheet<Error> Error = Service.DataManager.GetExcelSheet<Error>()!;
}
