using Dalamud.Interface.Utility.Raii;
using Dalamud.Interface;
using ImGuiNET;
using System.Numerics;
using System.Diagnostics;
using System;
using Dalamud.Interface.ManagedFontAtlas;

namespace Waitingway.Utils;

public static class ImGuiUtils
{
    private static Vector2 GetIconSize(FontAwesomeIcon icon)
    {
        using var font = ImRaii.PushFont(UiBuilder.IconFont);
        return ImGui.CalcTextSize(icon.ToIconString());
    }

    private static void DrawCenteredIcon(FontAwesomeIcon icon, Vector2 offset, Vector2 size, bool isDisabled = false)
    {
        var iconSize = GetIconSize(icon);

        float scale;
        Vector2 iconOffset;
        if (iconSize.X > iconSize.Y)
        {
            scale = size.X / iconSize.X;
            iconOffset = new(0, (size.Y - (iconSize.Y * scale)) / 2f);
        }
        else if (iconSize.Y > iconSize.X)
        {
            scale = size.Y / iconSize.Y;
            iconOffset = new((size.X - (iconSize.X * scale)) / 2f, 0);
        }
        else
        {
            scale = size.X / iconSize.X;
            iconOffset = Vector2.Zero;
        }

        ImGui.GetWindowDrawList().AddText(UiBuilder.IconFont, UiBuilder.IconFont.FontSize * scale, offset + iconOffset, ImGui.GetColorU32(!isDisabled ? ImGuiCol.Text : ImGuiCol.TextDisabled), icon.ToIconString());
    }

    public static bool IconButtonSquare(FontAwesomeIcon icon, float size = -1)
    {
        var ret = false;

        var buttonSize = new Vector2(size == -1 ? ImGui.GetFrameHeight() : size);
        var pos = ImGui.GetCursorScreenPos();
        var spacing = new Vector2(ImGui.GetStyle().FramePadding.Y);

        if (ImGui.Button($"###{icon.ToIconString()}", buttonSize))
            ret = true;

        var isDisabled = ImGuiInternals.GetItemFlags().HasFlag(ImGuiItemFlags.Disabled);
        DrawCenteredIcon(icon, pos + spacing, buttonSize - spacing * 2, isDisabled);

        return ret;
    }

    public static void AlignCentered(float width, float availWidth = default)
    {
        if (availWidth == default)
            availWidth = ImGui.GetContentRegionAvail().X;
        if (availWidth > width)
            ImGui.SetCursorPosX(ImGui.GetCursorPos().X + (availWidth - width) / 2);
    }

    public static void AlignRight(float width, float availWidth = default)
    {
        if (availWidth == default)
            availWidth = ImGui.GetContentRegionAvail().X;
        if (availWidth > width)
            ImGui.SetCursorPosX(ImGui.GetCursorPos().X + availWidth - width);
    }

    public static void AlignMiddle(Vector2 size, Vector2 availSize = default)
    {
        if (availSize == default)
            availSize = ImGui.GetContentRegionAvail();
        if (availSize.X > size.X)
            ImGui.SetCursorPosX(ImGui.GetCursorPos().X + (availSize.X - size.X) / 2);
        if (availSize.Y > size.Y)
            ImGui.SetCursorPosY(ImGui.GetCursorPos().Y + (availSize.Y - size.Y) / 2);
    }

    // https://stackoverflow.com/a/67855985
    public static void TextCentered(string text, float availWidth = default)
    {
        AlignCentered(ImGui.CalcTextSize(text).X, availWidth);
        ImGui.TextUnformatted(text);
    }

    public static void TextRight(string text, float availWidth = default)
    {
        AlignRight(ImGui.CalcTextSize(text).X, availWidth);
        ImGui.TextUnformatted(text);
    }

    // https://gist.github.com/dougbinks/ef0962ef6ebe2cadae76c4e9f0586c69#file-imguiutils-h-L219
    private static void UnderlineLastItem(Vector4 color)
    {
        var min = ImGui.GetItemRectMin();
        var max = ImGui.GetItemRectMax();
        min.Y = max.Y;
        ImGui.GetWindowDrawList().AddLine(min, max, ImGui.ColorConvertFloat4ToU32(color), 1);
    }

    // https://gist.github.com/dougbinks/ef0962ef6ebe2cadae76c4e9f0586c69#file-imguiutils-h-L228
    public static unsafe void Hyperlink(string text, string url, bool underline = true)
    {
        ImGui.TextUnformatted(text);
        if (underline)
            UnderlineLastItem(*ImGui.GetStyleColorVec4(ImGuiCol.Text));
        if (ImGui.IsItemHovered())
        {
            ImGui.SetMouseCursor(ImGuiMouseCursor.Hand);
            if (ImGui.IsItemClicked(ImGuiMouseButton.Left))
                Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true });
            var urlWithoutScheme = url;
            if (Uri.TryCreate(url, UriKind.Absolute, out var uri))
                urlWithoutScheme = uri.Host + (string.Equals(uri.PathAndQuery, "/", StringComparison.Ordinal) ? string.Empty : uri.PathAndQuery);
            Tooltip(urlWithoutScheme);
        }
    }

    public static void TextWrappedTo(string text, float wrapPosX = default, float basePosX = default)
    {
        var font = ImGui.GetFont();

        var currentPos = ImGui.GetCursorPosX();

        if (basePosX == default)
            basePosX = ImGui.GetCursorStartPos().X;

        float currentWrapWidth;
        if (wrapPosX == default)
            currentWrapWidth = ImGui.GetContentRegionAvail().X;
        else
            currentWrapWidth = wrapPosX - currentPos;

        var textBuf = text.AsSpan();
        var lineSize = font.CalcWordWrapPositionA(1, textBuf, currentWrapWidth) ?? textBuf.Length;
        var lineBuf = textBuf[..lineSize];
        ImGui.Text(lineBuf.ToString());
        var remainingBuf = textBuf[lineSize..].TrimStart();

        if (!remainingBuf.IsEmpty)
        {
            ImGui.SetCursorPosX(basePosX);
            using (ImRaii2.TextWrapPos(wrapPosX))
                ImGui.TextWrapped(remainingBuf.ToString());
        }
    }

    public static void Tooltip(string text)
    {
        using var _font = ImRaii.PushFont(UiBuilder.DefaultFont);
        using var _tooltip = ImRaii.Tooltip();
        ImGui.TextUnformatted(text);
    }

    public static void TooltipWrapped(string text, float width = 300)
    {
        using var _font = ImRaii.PushFont(UiBuilder.DefaultFont);
        using var _tooltip = ImRaii.Tooltip();
        using var _wrap = ImRaii2.TextWrapPos(width);
        ImGui.TextUnformatted(text);
    }

    public static float GetFontSize(this IFontHandle font)
    {
        using (font.Push())
            return ImGui.GetFontSize();
    }
}
