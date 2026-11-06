// SPDX-License-Identifier: AGPL-3.0-only
import { readdirSync, readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve as pathResolve, join } from 'node:path';
import { describe, test, expect } from 'vitest';
import { StdioTransport, UmweltError } from '../packages/umwelt/src/index.js';

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
function resolve(value: Json, captures: Map<string, Json>): Json {
  if (typeof value === 'string' && value.startsWith('$ref:')) {
    if (!captures.has(value.slice(5))) throw new Error(`Unknown capture ${value}`);
    return captures.get(value.slice(5))!;
  }
  if (Array.isArray(value)) return value.map(item => resolve(item, captures));
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, resolve(item, captures)]));
  return value;
}
function subset(expected: Json, actual: Json, captures: Map<string, Json>): void {
  if (expected === '$any') return;
  if (typeof expected === 'string' && expected.startsWith('$capture:')) { captures.set(expected.slice(9), actual); return; }
  if (typeof expected === 'string' && expected.startsWith('$ref:')) { expect(actual).toEqual(captures.get(expected.slice(5))); return; }
  if (Array.isArray(expected)) {
    expect(Array.isArray(actual)).toBe(true);
    expect((actual as Json[]).length).toBe(expected.length);
    expected.forEach((item, index) => subset(item, (actual as Json[])[index]!, captures));
  } else if (expected && typeof expected === 'object') {
    expect(actual).not.toBeNull();
    for (const [key, value] of Object.entries(expected)) {
      expect(actual).toHaveProperty(key);
      subset(value, (actual as Record<string, Json>)[key]!, captures);
    }
  } else expect(actual).toEqual(expected);
}
describe('shared protocol conformance', () => {
  const directory = pathResolve('conformance/cases');
  for (const file of readdirSync(directory).filter(file => file.endsWith('.json')).sort()) {
    const value = JSON.parse(readFileSync(join(directory, file), 'utf8')) as { name: string; steps: { request: { op: string; payload: Json }; expect: Json }[] };
    test(value.name, async () => {
      const temporary = mkdtempSync(join(tmpdir(), 'umwelt-conformance-'));
      const transport = new StdioTransport({ file: join(temporary, 'test.umw'), create: true, binary: pathResolve(`target/debug/umwelt${process.platform === 'win32' ? '.exe' : ''}`), automaticHello: false });
      const captures = new Map<string, Json>();
      try {
        for (const step of value.steps) {
          const payload = resolve(step.request.payload, captures);
          let response: Json;
          try { response = { ok: true, result: await transport.request<Json>(step.request.op, payload) }; }
          catch (error) {
            if (!(error instanceof UmweltError)) throw error;
            response = { ok: false, error: { code: error.code, message: error.message, ...(error.details ? { details: error.details as Json } : {}) } };
          }
          subset(step.expect, response, captures);
        }
      } finally { await transport.close(); rmSync(temporary, { recursive: true, force: true }); }
    });
  }
});
