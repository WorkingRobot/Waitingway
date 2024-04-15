using Dalamud.Interface.Utility.Raii;
using ImGuiNET;
using System;

namespace Waitingway.Utils;

public static class ImRaii2
{
    private struct EndUnconditionally(Action endAction, bool success) : ImRaii.IEndObject, IDisposable
    {
        private Action EndAction { get; } = endAction;

        public bool Success { get; } = success;

        public bool Disposed { get; private set; } = false;

        public void Dispose()
        {
            if (!Disposed)
            {
                EndAction();
                Disposed = true;
            }
        }
    }

    private struct EndConditionally(Action endAction, bool success) : ImRaii.IEndObject, IDisposable
    {
        public bool Success { get; } = success;

        public bool Disposed { get; private set; } = false;

        private Action EndAction { get; } = endAction;

        public void Dispose()
        {
            if (!Disposed)
            {
                if (Success)
                {
                    EndAction();
                }

                Disposed = true;
            }
        }
    }

    public static ImRaii.IEndObject TextWrapPos(float wrap_local_pos_x)
    {
        ImGui.PushTextWrapPos(wrap_local_pos_x);
        return new EndUnconditionally(ImGui.PopTextWrapPos, true);
    }
}
