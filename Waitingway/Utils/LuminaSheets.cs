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
    public static readonly ExcelSheet<LogMessage> LogMessage = Module.GetSheet<LogMessage>()!;
    public static readonly ExcelSheet<ClassJob> ClassJob = Module.GetSheet<ClassJob>()!;

    public static RowRef<T> CreateRowRef<T>(uint row) where T : struct, IExcelRow<T> =>
        new(Module, row);
}
