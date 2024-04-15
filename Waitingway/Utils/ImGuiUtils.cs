using Dalamud.Interface.Utility.Raii;
using Dalamud.Interface;
using ImGuiNET;

namespace Waitingway.Utils;

public static class ImGuiUtils
{
    public static void TooltipWrapped(string text, float width = 300)
    {
        using var _font = ImRaii.PushFont(UiBuilder.DefaultFont);
        using var _tooltip = ImRaii.Tooltip();
        using var _wrap = ImRaii2.TextWrapPos(width);
        ImGui.TextUnformatted(text);
    }
}
