// SPDX-License-Identifier: AGPL-3.0-only
using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Kallamo.Umwelt;

/// <summary>
/// Where the binding finds the native library (<c>umwelt_ffi</c>).
/// </summary>
/// <remarks>
/// The library is looked up, in order: at <see cref="LibraryPath"/> when set;
/// next to <c>Kallamo.Umwelt.dll</c>; in <c>runtimes/&lt;rid&gt;/native</c> next to
/// <c>Kallamo.Umwelt.dll</c>; then through the default .NET lookup. Hosts that load
/// plugins from their own folders (game mod loaders, for example) therefore
/// need no configuration when the library ships beside <c>Kallamo.Umwelt.dll</c>.
/// </remarks>
public static class UmweltNative
{
    private static readonly object Gate = new();
    private static string? libraryPath;
    private static string? loadedFrom;
    private static bool registered;

    /// <summary>
    /// A native library file, or a directory to search for it. Must be set
    /// before the first engine is opened; once the library is loaded it can
    /// no longer change.
    /// </summary>
    public static string? LibraryPath
    {
        get
        {
            lock (Gate)
            {
                return libraryPath;
            }
        }
        set
        {
            lock (Gate)
            {
                if (loadedFrom is not null)
                {
                    throw new InvalidOperationException($"The native library is already loaded from '{loadedFrom}'.");
                }
                libraryPath = value;
            }
        }
    }

    /// <summary>
    /// The file the native library was loaded from, or <c>null</c> before it
    /// is loaded or when the default .NET lookup found it.
    /// </summary>
    public static string? LoadedFrom
    {
        get
        {
            lock (Gate)
            {
                return loadedFrom;
            }
        }
    }

    /// <summary>The file name of the native library on this platform.</summary>
    public static string FileName =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "umwelt_ffi.dll"
        : RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "libumwelt_ffi.dylib"
        : "libumwelt_ffi.so";

    /// <summary>Installs the resolver once, before the first native call.</summary>
    internal static void Register()
    {
        lock (Gate)
        {
            if (registered)
            {
                return;
            }
            registered = true;
        }
        try
        {
            NativeLibrary.SetDllImportResolver(typeof(UmweltNative).Assembly, Resolve);
        }
        catch (InvalidOperationException)
        {
            // The host installed its own resolver for this assembly; it wins.
        }
    }

    private static IntPtr Resolve(string name, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (name != NativeMethods.Library)
        {
            return IntPtr.Zero;
        }
        lock (Gate)
        {
            foreach (var candidate in Candidates(assembly))
            {
                if (File.Exists(candidate) && NativeLibrary.TryLoad(candidate, out var library))
                {
                    loadedFrom = candidate;
                    return library;
                }
            }
            if (libraryPath is not null)
            {
                throw new DllNotFoundException($"Umwelt's native library ({FileName}) was not found at '{libraryPath}'.");
            }
            // Fall back to the default lookup.
            return IntPtr.Zero;
        }
    }

    private static IEnumerable<string> Candidates(Assembly assembly)
    {
        if (libraryPath is not null)
        {
            if (Directory.Exists(libraryPath))
            {
                foreach (var candidate in InDirectory(libraryPath))
                {
                    yield return candidate;
                }
            }
            else
            {
                yield return libraryPath;
            }
            yield break;
        }
        // Empty when the assembly was loaded from bytes.
        var directory = string.IsNullOrEmpty(assembly.Location) ? null : Path.GetDirectoryName(assembly.Location);
        if (directory is not null)
        {
            foreach (var candidate in InDirectory(directory))
            {
                yield return candidate;
            }
        }
    }

    private static IEnumerable<string> InDirectory(string directory)
    {
        yield return Path.Combine(directory, FileName);
        if (RuntimeIdentifier() is { } rid)
        {
            yield return Path.Combine(directory, "runtimes", rid, "native", FileName);
        }
    }

    /// <summary>The runtime identifier the NuGet package uses for this machine.</summary>
    private static string? RuntimeIdentifier()
    {
        var os = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "win"
            : RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "osx"
            : RuntimeInformation.IsOSPlatform(OSPlatform.Linux) ? "linux"
            : null;
        var architecture = RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            Architecture.X86 => "x86",
            _ => null,
        };
        return os is null || architecture is null ? null : $"{os}-{architecture}";
    }
}
