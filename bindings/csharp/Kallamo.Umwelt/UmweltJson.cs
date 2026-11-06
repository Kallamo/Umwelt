// SPDX-License-Identifier: AGPL-3.0-only
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Kallamo.Umwelt;

/// <summary>JSON settings matching the protocol's wire format.</summary>
internal static partial class UmweltJson
{
    internal static readonly JsonSerializerOptions Options = CreateOptions();

    private static JsonSerializerOptions CreateOptions()
    {
        var options = new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        };
        foreach (var converter in GeneratedConverters())
        {
            options.Converters.Add(converter);
        }
        return options;
    }
}
