// SPDX-License-Identifier: AGPL-3.0-only
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const model = JSON.parse(readFileSync(resolve(root, 'spec/schema/umwelt-v1.schema.json'), 'utf8'));
const protocol = JSON.parse(readFileSync(resolve(root, 'spec/schema/umwelt-protocol-v1.schema.json'), 'utf8'));
const definitions = { ...model.$defs, ...protocol.$defs };
const reference = ref => ref.split('/').at(-1);
function type(schema) {
  if (schema === true) return 'unknown';
  if (schema === false) return 'never';
  if (schema.$ref) return reference(schema.$ref);
  if (schema.const !== undefined) return JSON.stringify(schema.const);
  if (schema.enum) return schema.enum.map(value => JSON.stringify(value)).join(' | ');
  if (schema.oneOf || schema.anyOf) return (schema.oneOf ?? schema.anyOf).map(item => `(${type(item)})`).join(' | ');
  if (schema.allOf) return schema.allOf.map(item => `(${type(item)})`).join(' & ');
  if (Array.isArray(schema.type)) return schema.type.map(kind => type({ ...schema, type: kind })).join(' | ');
  switch (schema.type) {
    case 'string': return 'string';
    case 'integer': case 'number': return 'number';
    case 'boolean': return 'boolean';
    case 'null': return 'null';
    case 'array': return `Array<${type(schema.items ?? true)}>`;
    case 'object': {
      const fields = Object.entries(schema.properties ?? {}).map(([key, value]) => `${JSON.stringify(key)}${schema.required?.includes(key) ? '' : '?'}: ${type(value)};`);
      if (schema.additionalProperties && schema.additionalProperties !== false) fields.push(`[key: string]: ${schema.properties ? 'unknown' : type(schema.additionalProperties)};`);
      return `{ ${fields.join(' ')} }`;
    }
    default: return 'unknown';
  }
}
const banner = '// SPDX-License-Identifier: AGPL-3.0-only\n// Generated from the versioned JSON schemas. Do not edit.\n';
const contracts = banner + Object.entries(definitions).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([name, schema]) => `export type ${name} = ${type(schema)};`).join('\n') + '\n';
const operations = Object.entries(protocol['x-ops']);
const methods = operations.map(([op, schema]) => {
  const method = op.replace(/\.([a-z])/g, (_, letter) => letter.toUpperCase());
  return `  ${method}(payload: T.${reference(schema.payload)}): Promise<T.${reference(schema.result)}> { return this.transport.request(${JSON.stringify(op)}, payload); }`;
}).join('\n');
const client = `${banner}import type * as T from './contracts.js';\nimport type { StdioTransport } from '../transport.js';\nexport class UmweltClient {\n  constructor(readonly transport: StdioTransport) {}\n${methods}\n  close(): Promise<void> { return this.transport.close(); }\n}\n`;
const directory = resolve(root, 'packages/umwelt/src/generated');
mkdirSync(directory, { recursive: true });
for (const [name, output] of [['contracts.ts', contracts], ['client.ts', client]]) {
  const path = resolve(directory, name);
  if (process.argv.includes('--check')) {
    if (readFileSync(path, 'utf8') !== output) throw new Error(`Generated types drift: ${name}. Run npm run generate:types.`);
  } else writeFileSync(path, output);
}
