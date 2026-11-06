// SPDX-License-Identifier: AGPL-3.0-only
using System;
using System.Text.Json;

namespace Kallamo.Umwelt;

/// <summary>
/// An error returned by the engine. <see cref="Code"/> is the stable protocol
/// code (for example <c>VERSION_CONFLICT</c>); <see cref="Details"/> carries
/// its machine-readable details when the engine sends them.
/// </summary>
public class UmweltException : Exception
{
    public UmweltException(string code, string message, JsonElement? details = null)
        : base(message)
    {
        Code = code;
        Details = details;
    }

    /// <summary>The stable protocol error code.</summary>
    public string Code { get; }

    /// <summary>Machine-readable details, when present.</summary>
    public JsonElement? Details { get; }

    internal static UmweltException From(string code, string message, JsonElement? details) => code switch
    {
        "VALIDATION_FAILED" => new ValidationFailedException(message, details),
        "NOT_FOUND" => new NotFoundException(message, details),
        "VERSION_CONFLICT" => new VersionConflictException(message, details),
        "CONFLICT" => new ConflictException(message, details),
        "INVALID_STATE" => new InvalidStateException(message, details),
        "FILE_LOCKED" => new FileLockedException(message, details),
        "NOT_UMWELT_FILE" or "UNSUPPORTED_FILE_VERSION" => new UnsupportedFileException(code, message, details),
        _ => new UmweltException(code, message, details),
    };
}

/// <summary>The request is invalid (<c>VALIDATION_FAILED</c>).</summary>
public sealed class ValidationFailedException : UmweltException
{
    public ValidationFailedException(string message, JsonElement? details = null)
        : base("VALIDATION_FAILED", message, details)
    {
    }
}

/// <summary>Something referenced does not exist (<c>NOT_FOUND</c>).</summary>
public sealed class NotFoundException : UmweltException
{
    public NotFoundException(string message, JsonElement? details = null)
        : base("NOT_FOUND", message, details)
    {
    }
}

/// <summary>
/// The expected version is no longer current (<c>VERSION_CONFLICT</c>), for
/// example a draft promoted after main moved.
/// </summary>
public sealed class VersionConflictException : UmweltException
{
    public VersionConflictException(string message, JsonElement? details = null)
        : base("VERSION_CONFLICT", message, details)
    {
    }
}

/// <summary>An id or attempt is already used differently (<c>CONFLICT</c>).</summary>
public sealed class ConflictException : UmweltException
{
    public ConflictException(string message, JsonElement? details = null)
        : base("CONFLICT", message, details)
    {
    }
}

/// <summary>The operation does not apply in the current state (<c>INVALID_STATE</c>).</summary>
public sealed class InvalidStateException : UmweltException
{
    public InvalidStateException(string message, JsonElement? details = null)
        : base("INVALID_STATE", message, details)
    {
    }
}

/// <summary>Another process has the file open (<c>FILE_LOCKED</c>).</summary>
public sealed class FileLockedException : UmweltException
{
    public FileLockedException(string message, JsonElement? details = null)
        : base("FILE_LOCKED", message, details)
    {
    }
}

/// <summary>
/// The file is not an Umwelt file, or was written by another storage version
/// (<c>NOT_UMWELT_FILE</c>, <c>UNSUPPORTED_FILE_VERSION</c>).
/// </summary>
public sealed class UnsupportedFileException : UmweltException
{
    public UnsupportedFileException(string code, string message, JsonElement? details = null)
        : base(code, message, details)
    {
    }
}

/// <summary>
/// The native library itself failed: an invalid argument, a failed open, a
/// panic, or a handle poisoned by an earlier panic. <see cref="Status"/> is
/// the C ABI status.
/// </summary>
public sealed class UmweltNativeException : UmweltException
{
    public UmweltNativeException(int status, string message)
        : base(StatusName(status), message)
    {
        Status = status;
    }

    /// <summary>The C ABI status (1 invalid argument, 2 open failed, 3 poisoned, 4 panic).</summary>
    public int Status { get; }

    private static string StatusName(int status) => status switch
    {
        1 => "INVALID_ARGUMENT",
        2 => "OPEN_FAILED",
        3 => "POISONED",
        4 => "PANIC",
        _ => "NATIVE_ERROR",
    };
}
