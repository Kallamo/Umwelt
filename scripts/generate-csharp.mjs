// SPDX-License-Identifier: AGPL-3.0-only
// Generates the C# contract types and the typed client of the official .NET
// binding from the versioned JSON schemas. Usage: node generate-csharp.mjs [--check]
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const model = JSON.parse(readFileSync(resolve(root, 'spec/schema/umwelt-v1.schema.json'), 'utf8'));
const protocol = JSON.parse(readFileSync(resolve(root, 'spec/schema/umwelt-protocol-v1.schema.json'), 'utf8'));
const definitions = { ...model.$defs, ...protocol.$defs };
// Envelopes and the handshake are handled by the runtime, not by user code.
const internal = new Set(['RequestEnvelope', 'SuccessEnvelope', 'FailureEnvelope', 'ResponseEnvelope', 'HelloPayload', 'HelloResult', 'ProtocolRange', 'Capability', 'CapabilityKind', 'PublicError', 'ErrorCode']);

const pascal = text => text.split(/[^A-Za-z0-9]+/).filter(Boolean).map(part => part[0].toUpperCase() + part.slice(1)).join('');
const reference = ref => ref.split('/').at(-1);
const isTagged = schema => Array.isArray(schema.oneOf) && schema.oneOf.every(variant => variant.properties?.kind?.const !== undefined);
// A string enum, written either as `enum` or, when its variants are
// documented, as `oneOf` of string constants.
const enumValues = schema => {
  if (Array.isArray(schema.enum) && schema.enum.every(value => typeof value === 'string')) return schema.enum.map(value => ({ value }));
  if (Array.isArray(schema.oneOf) && schema.oneOf.every(item => !item.properties && (typeof item.const === 'string' || Array.isArray(item.enum)))) {
    return schema.oneOf.flatMap(item => (item.enum ?? [item.const]).map(value => ({ value, description: item.description })));
  }
  return null;
};
const isEnum = schema => enumValues(schema) !== null;
const isArrayAlias = schema => schema.type === 'array';
const isMap = schema => schema.type === 'object' && !schema.properties && schema.additionalProperties !== undefined && schema.additionalProperties !== false;
const valueTypes = new Set(['bool', 'long', 'ulong', 'int', 'uint', 'double', 'JsonElement']);

function integer(format) {
  return { uint64: 'ulong', int64: 'long', uint32: 'uint', int32: 'int', uint: 'ulong', uint8: 'byte', uint16: 'ushort' }[format] ?? 'long';
}

// C# type of a schema, and whether it is a value type.
function csType(schema) {
  if (schema === true || (schema && Object.keys(schema).length === 0)) return { name: 'JsonElement', value: true };
  if (schema.$ref) {
    const name = reference(schema.$ref);
    const target = definitions[name];
    if (target && isArrayAlias(target)) return { name: `List<${csType(target.items ?? true).name}>`, value: false };
    if (target && isMap(target)) return csType(target);
    return { name, value: target ? isEnum(target) : false };
  }
  const variants = schema.anyOf ?? schema.oneOf;
  if (variants && variants.length === 2 && variants.some(item => item.type === 'null')) {
    const inner = csType(variants.find(item => item.type !== 'null'));
    return { ...inner, nullable: true };
  }
  if (Array.isArray(schema.type)) {
    const inner = csType({ ...schema, type: schema.type.find(kind => kind !== 'null') });
    return { ...inner, nullable: schema.type.includes('null') };
  }
  if (schema.const !== undefined) return { name: 'string', value: false };
  switch (schema.type) {
    case 'string': return { name: 'string', value: false };
    case 'integer': return { name: integer(schema.format), value: true };
    case 'number': return { name: 'double', value: true };
    case 'boolean': return { name: 'bool', value: true };
    case 'array': return { name: `List<${csType(schema.items ?? true).name}>`, value: false };
    case 'object':
      if (isMap(schema)) return { name: `Dictionary<string, ${csType(schema.additionalProperties === true ? true : schema.additionalProperties).name}>`, value: false };
      return { name: 'JsonElement', value: true };
    default: return { name: 'JsonElement', value: true };
  }
}

function summary(text, indent) {
  if (!text) return '';
  const lines = text.split('\n').map(line => line.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;'));
  return `${indent}/// <summary>\n${lines.map(line => `${indent}/// ${line}`.trimEnd()).join('\n')}\n${indent}/// </summary>\n`;
}

function property(json, schema, required, indent, owner) {
  const type = csType(schema);
  let name = pascal(json);
  if (name === owner) name += 'Value';
  const optional = !required || type.nullable;
  const declared = optional ? `${type.name}?` : type.name;
  const ignore = optional ? `${indent}[JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]\n` : '';
  let initializer = '';
  if (!optional && !type.value) initializer = type.name.startsWith('List<') || type.name.startsWith('Dictionary<') ? ' = new();' : ' = default!;';
  return `${summary(schema.description, indent)}${ignore}${indent}[JsonPropertyName("${json}")]\n${indent}public ${declared} ${name} { get; set; }${initializer}\n`;
}

function members(schema, indent, owner, skip = new Set()) {
  const required = new Set(schema.required ?? []);
  return Object.entries(schema.properties ?? {})
    .filter(([json]) => !skip.has(json))
    .map(([json, value]) => property(json, value, required.has(json), indent, owner))
    .join('\n');
}

const converters = [];

function enumDefinition(name, schema) {
  const values = enumValues(schema);
  const cases = values.map(({ value }) => [pascal(value), value]);
  converters.push(`${name}Converter`);
  const members = values.map(({ value, description }) => `${summary(description, '    ')}    ${pascal(value)},`).join('\n');
  return `${summary(schema.description, '')}public enum ${name}\n{\n${members}\n}\n\n` +
    `internal sealed class ${name}Converter : JsonConverter<${name}>\n{\n` +
    `    public override ${name} Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch\n    {\n` +
    cases.map(([member, value]) => `        "${value}" => ${name}.${member},\n`).join('') +
    `        var other => throw new JsonException($"Unknown ${name} '{other}'."),\n    };\n\n` +
    `    public override void Write(Utf8JsonWriter writer, ${name} value, JsonSerializerOptions options) => writer.WriteStringValue(value switch\n    {\n` +
    cases.map(([member, value]) => `        ${name}.${member} => "${value}",\n`).join('') +
    `        _ => throw new JsonException($"Unknown ${name} {value}."),\n    });\n}\n`;
}

function taggedDefinition(name, schema) {
  const variants = schema.oneOf.map(variant => {
    const kind = variant.properties.kind.const;
    let className = pascal(kind);
    const collides = Object.keys(variant.properties).some(json => pascal(json) === className);
    if (collides || className === name) className += name;
    return { kind, className, variant };
  });
  converters.push(`${name}Converter`);
  const body = variants.map(({ kind, className, variant }) =>
    `${summary(variant.description, '    ')}    public sealed class ${className} : ${name}\n    {\n` +
    `        [JsonPropertyName("kind")]\n        public override string Kind => "${kind}";\n\n` +
    members(variant, '        ', className, new Set(['kind'])) + `    }\n`).join('\n');
  return `${summary(schema.description, '')}public abstract class ${name}\n{\n    /// <summary>The variant tag.</summary>\n    [JsonPropertyName("kind")]\n    public abstract string Kind { get; }\n\n${body}}\n\n` +
    `internal sealed class ${name}Converter : JsonConverter<${name}>\n{\n` +
    `    public override ${name} Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)\n    {\n` +
    `        using var document = JsonDocument.ParseValue(ref reader);\n` +
    `        var kind = document.RootElement.TryGetProperty("kind", out var tag) ? tag.GetString() : null;\n` +
    `        return kind switch\n        {\n` +
    variants.map(({ kind, className }) => `            "${kind}" => document.RootElement.Deserialize<${name}.${className}>(options)!,\n`).join('') +
    `            _ => throw new JsonException($"Unknown ${name} kind '{kind}'."),\n        };\n    }\n\n` +
    `    public override void Write(Utf8JsonWriter writer, ${name} value, JsonSerializerOptions options) => JsonSerializer.Serialize(writer, value, value.GetType(), options);\n}\n`;
}

function objectDefinition(name, schema) {
  // An untagged union (ClaimRef) becomes one class with every variant's
  // properties, all optional.
  if (schema.anyOf && !schema.properties) {
    const merged = { properties: Object.assign({}, ...schema.anyOf.map(item => item.properties ?? {})), required: [] };
    return `${summary(schema.description ?? 'Exactly one of the properties is set.', '')}public sealed class ${name}\n{\n${members(merged, '    ', name)}}\n`;
  }
  return `${summary(schema.description, '')}public sealed class ${name}\n{\n${members(schema, '    ', name)}}\n`;
}

const banner = '// SPDX-License-Identifier: AGPL-3.0-only\n// Generated from the versioned JSON schemas by scripts/generate-csharp.mjs. Do not edit.\n#nullable enable\n';
const types = [];
for (const [name, schema] of Object.entries(definitions).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
  if (internal.has(name) || isArrayAlias(schema) || isMap(schema)) continue;
  if (isEnum(schema)) types.push(enumDefinition(name, schema));
  else if (isTagged(schema)) types.push(taggedDefinition(name, schema));
  else types.push(objectDefinition(name, schema));
}
const contracts = `${banner}using System;\nusing System.Collections.Generic;\nusing System.Text.Json;\nusing System.Text.Json.Serialization;\n\nnamespace Kallamo.Umwelt;\n\n${types.join('\n')}\n` +
  `internal static partial class UmweltJson\n{\n    private static IEnumerable<JsonConverter> GeneratedConverters() => new JsonConverter[]\n    {\n${converters.sort().map(converter => `        new ${converter}(),`).join('\n')}\n    };\n}\n`;

const operations = Object.entries(protocol['x-ops']).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
const methods = operations.map(([op, schema]) => {
  const payload = csType({ $ref: schema.payload }).name;
  const result = csType({ $ref: schema.result }).name;
  return `    /// <summary>Protocol operation <c>${op}</c> (${schema.kind}).</summary>\n    public ${result} ${pascal(op)}(${payload} payload) => Call<${result}>("${op}", payload);\n`;
}).join('\n');
const client = `${banner}using System.Collections.Generic;\n\nnamespace Kallamo.Umwelt;\n\npublic sealed partial class UmweltEngine\n{\n${methods}}\n`;

const directory = resolve(root, 'bindings/csharp/Kallamo.Umwelt/Generated');
mkdirSync(directory, { recursive: true });
for (const [name, output] of [['Contracts.cs', contracts], ['Client.cs', client]]) {
  const path = resolve(directory, name);
  if (process.argv.includes('--check')) {
    let current = '';
    try { current = readFileSync(path, 'utf8'); } catch { /* missing */ }
    if (current !== output) throw new Error(`Generated C# drift: ${name}. Run npm run generate:csharp.`);
  } else writeFileSync(path, output);
}
