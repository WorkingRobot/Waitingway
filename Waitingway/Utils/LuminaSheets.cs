using Lumina.Excel;
using Lumina.Excel.GeneratedSheets2;
using LWorld = Lumina.Excel.GeneratedSheets2.World;

namespace Waitingway.Utils;

public static class LuminaSheets
{
    public static readonly ExcelSheet<LWorld> World = Service.DataManager.GetExcelSheet<LWorld>()!;
    public static readonly ExcelSheet<WorldDCGroupType> WorldDCGroupType = Service.DataManager.GetExcelSheet<WorldDCGroupType>()!;
    public static readonly ExcelSheet<Error> Error = Service.DataManager.GetExcelSheet<Error>()!;
}
