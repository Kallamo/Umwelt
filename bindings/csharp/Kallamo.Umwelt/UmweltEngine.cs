// SPDX-License-Identifier: AGPL-3.0-only
using System;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

namespace Kallamo.Umwelt;

/// <summary>How hard SQLite works to keep committed transactions.</summary>
public enum Durability
{
    /// <summary>
    /// Survives an application crash; a power loss may drop the most recent
    /// commits. The file is never corrupted. Default.
    /// </summary>
    Normal = 0,

    /// <summary>Every commit is flushed to disk before it returns.</summary>
    Full = 1,
}

/// <summary>
/// One open <c>.umw</c> file, running in-process through the native library.
/// Every protocol operation is a typed method (see the generated part of this
/// class). Calls on one engine are serialized; dispose it to close the file.
/// </summary>
public sealed partial class UmweltEngine : IDisposable
{
    private readonly UmweltHandle handle;
    private readonly object gate = new();
    private long nextId;

    private UmweltEngine(UmweltHandle handle)
    {
        this.handle = handle;
    }

    static UmweltEngine()
    {
        // Every native call goes through this class, so the library is
        // resolved by Umwelt's own lookup (see UmweltNative).
        UmweltNative.Register();
    }

    /// <summary>The C ABI version of the loaded native library.</summary>
    public static uint NativeAbiVersion => NativeMethods.umwelt_abi_version();

    /// <summary>Opens an existing <c>.umw</c> file.</summary>
    public static UmweltEngine Open(string path, Durability durability = Durability.Normal, string client = "umwelt-dotnet")
        => Start(path, create: false, durability, client);

    /// <summary>Creates a new <c>.umw</c> file. Fails if it already exists.</summary>
    public static UmweltEngine Create(string path, Durability durability = Durability.Normal, string client = "umwelt-dotnet")
        => Start(path, create: true, durability, client);

    private static UmweltEngine Start(string path, bool create, Durability durability, string client)
    {
        var abi = NativeMethods.umwelt_abi_version();
        if (abi != NativeMethods.SupportedAbiVersion)
        {
            throw new UmweltNativeException(0, $"Native ABI version {abi} is not supported; expected {NativeMethods.SupportedAbiVersion}.");
        }
        var bytes = Encoding.UTF8.GetBytes(path);
        var status = NativeMethods.umwelt_open_with_options(bytes, (UIntPtr)bytes.Length, create ? 1 : 0, (int)durability, out var handle);
        if (status != 0)
        {
            var error = handle.IsInvalid ? "{}" : LastError(handle);
            handle.Dispose();
            throw OpenFailure(status, error);
        }
        var engine = new UmweltEngine(handle);
        try
        {
            engine.Hello(client);
        }
        catch
        {
            engine.Dispose();
            throw;
        }
        return engine;
    }

    /// <summary>
    /// Opens without the protocol handshake, for the conformance runner, which
    /// sends <c>hello</c> itself.
    /// </summary>
    internal static UmweltEngine OpenWithoutHandshake(string path, bool create)
    {
        var bytes = Encoding.UTF8.GetBytes(path);
        var status = NativeMethods.umwelt_open_with_options(bytes, (UIntPtr)bytes.Length, create ? 1 : 0, 0, out var handle);
        if (status != 0)
        {
            var error = handle.IsInvalid ? "{}" : LastError(handle);
            handle.Dispose();
            throw OpenFailure(status, error);
        }
        return new UmweltEngine(handle);
    }

    /// <summary>
    /// Sends one request and returns the whole response envelope without
    /// turning errors into exceptions, for the conformance runner.
    /// </summary>
    internal JsonElement SendEnvelope(string op, JsonElement payload)
    {
        lock (gate)
        {
            var id = Interlocked.Increment(ref nextId).ToString(System.Globalization.CultureInfo.InvariantCulture);
            var request = JsonSerializer.SerializeToUtf8Bytes(new Request(1, id, op, payload));
            var status = NativeMethods.umwelt_call(handle, request, (UIntPtr)request.Length, out var output);
            if (status != 0)
            {
                throw new UmweltNativeException(status, LastError(handle));
            }
            using var document = JsonDocument.Parse(NativeMethods.TakeString(output));
            return document.RootElement.Clone();
        }
    }

    /// <summary>Sends one operation with a raw JSON payload and returns the raw result.</summary>
    public JsonElement CallRaw(string op, JsonElement payload) => Call<JsonElement>(op, payload);

    /// <summary>Sends one operation and deserializes its result.</summary>
    public TResult Call<TResult>(string op, object payload)
    {
        var response = Send(op, JsonSerializer.SerializeToElement(payload, payload.GetType(), UmweltJson.Options));
        return response.Result.Deserialize<TResult>(UmweltJson.Options)!;
    }

    /// <summary>Checkpoints and closes the file.</summary>
    public void Dispose() => handle.Dispose();

    private void Hello(string client)
    {
        var payload = JsonSerializer.SerializeToElement(new { protocol = new { min = 1, max = 1 }, client });
        Send("hello", payload);
    }

    private Envelope Send(string op, JsonElement payload)
    {
        lock (gate)
        {
            if (handle.IsClosed)
            {
                throw new ObjectDisposedException(nameof(UmweltEngine));
            }
            var id = Interlocked.Increment(ref nextId).ToString(System.Globalization.CultureInfo.InvariantCulture);
            var request = JsonSerializer.SerializeToUtf8Bytes(new Request(1, id, op, payload));
            var status = NativeMethods.umwelt_call(handle, request, (UIntPtr)request.Length, out var output);
            if (status != 0)
            {
                throw new UmweltNativeException(status, LastError(handle));
            }
            var envelope = JsonSerializer.Deserialize<Envelope>(NativeMethods.TakeString(output))
                ?? throw new UmweltException("INTERNAL", "The engine returned an empty response.");
            if (!envelope.Ok)
            {
                var error = envelope.Error ?? new ErrorBody("INTERNAL", "The engine returned an error without details.", null);
                throw UmweltException.From(error.Code, error.Message, error.Details);
            }
            return envelope;
        }
    }

    private static string LastError(UmweltHandle handle)
    {
        return NativeMethods.umwelt_last_error(handle, out var buffer) == 0
            ? NativeMethods.TakeString(buffer)
            : "{}";
    }

    private static UmweltException OpenFailure(int status, string errorJson)
    {
        try
        {
            var error = JsonSerializer.Deserialize<ErrorBody>(errorJson);
            if (error is not null && !string.IsNullOrEmpty(error.Code))
            {
                return UmweltException.From(error.Code, error.Message, error.Details);
            }
        }
        catch (JsonException)
        {
            // Fall through to the native status.
        }
        return new UmweltNativeException(status, errorJson);
    }

    private sealed record Request(
        [property: JsonPropertyName("v")] int V,
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("op")] string Op,
        [property: JsonPropertyName("payload")] JsonElement Payload);

    private sealed record Envelope(
        [property: JsonPropertyName("ok")] bool Ok,
        [property: JsonPropertyName("result")] JsonElement Result,
        [property: JsonPropertyName("error")] ErrorBody? Error);

    private sealed record ErrorBody(
        [property: JsonPropertyName("code")] string Code,
        [property: JsonPropertyName("message")] string Message,
        [property: JsonPropertyName("details")] JsonElement? Details);
}
