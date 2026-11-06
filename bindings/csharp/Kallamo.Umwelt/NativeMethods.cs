// SPDX-License-Identifier: AGPL-3.0-only
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Kallamo.Umwelt;

/// <summary>A buffer owned by the native library; freed with <c>umwelt_buf_free</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
internal struct NativeBuffer
{
    public IntPtr Ptr;
    public UIntPtr Len;
}

/// <summary>The C ABI of <c>umwelt_ffi</c> (see <c>crates/umwelt-ffi/include/umwelt.h</c>).</summary>
internal static class NativeMethods
{
    internal const string Library = "umwelt_ffi";

    internal const uint SupportedAbiVersion = 1;

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern uint umwelt_abi_version();

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int umwelt_open_with_options(
        byte[] path,
        UIntPtr pathLength,
        int create,
        int durability,
        out UmweltHandle engine);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int umwelt_call(
        UmweltHandle engine,
        byte[] request,
        UIntPtr requestLength,
        out NativeBuffer output);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int umwelt_last_error(UmweltHandle engine, out NativeBuffer output);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void umwelt_buf_free(NativeBuffer buffer);

    [DllImport(Library, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void umwelt_close(IntPtr engine);

    /// <summary>Copies a native buffer into a string and frees it.</summary>
    internal static string TakeString(NativeBuffer buffer)
    {
        try
        {
            if (buffer.Ptr == IntPtr.Zero || buffer.Len == UIntPtr.Zero)
            {
                return string.Empty;
            }
            var bytes = new byte[checked((int)buffer.Len.ToUInt64())];
            Marshal.Copy(buffer.Ptr, bytes, 0, bytes.Length);
            return Encoding.UTF8.GetString(bytes);
        }
        finally
        {
            umwelt_buf_free(buffer);
        }
    }
}

/// <summary>An open engine handle. Closing it checkpoints the file.</summary>
internal sealed class UmweltHandle : SafeHandle
{
    public UmweltHandle()
        : base(IntPtr.Zero, ownsHandle: true)
    {
    }

    public override bool IsInvalid => handle == IntPtr.Zero;

    protected override bool ReleaseHandle()
    {
        NativeMethods.umwelt_close(handle);
        return true;
    }
}
