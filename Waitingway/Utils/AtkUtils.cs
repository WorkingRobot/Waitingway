using FFXIVClientStructs.FFXIV.Client.System.Memory;
using FFXIVClientStructs.FFXIV.Component.GUI;
using System.Numerics;

namespace Waitingway.Utils;

internal static unsafe class AtkUtils
{
    public static Vector2 GetPosition(this AtkResNode node)
    {
        Vector2 ret;
        node.GetPositionFloat(&ret.X, &ret.Y);
        return ret;
    }

    public static void SetPosition(this AtkResNode node, Vector2 pos)
    {
        node.SetPositionFloat(pos.X, pos.Y);
    }

    public static T* Calloc<T>() where T : unmanaged
    {
        var ptr = (T*)IMemorySpace.GetUISpace()->Malloc<T>();
        if (ptr == null)
            return null;

        IMemorySpace.Memset(ptr, 0, (ulong)sizeof(T));
        return ptr;
    }
}
