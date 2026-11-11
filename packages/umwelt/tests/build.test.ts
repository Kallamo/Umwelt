// SPDX-License-Identifier: AGPL-3.0-only
// The write factories against `conformance/builders.json`, which the .NET
// binding checks too: both languages must build the same JSON.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from 'vitest';
import * as build from '../src/build.js';

type Args = Record<string, unknown>;
interface Line { call: string; args: Args; expect: unknown }

const lines = JSON.parse(readFileSync(resolve('conformance/builders.json'), 'utf8')) as Line[];

/** A factory and the names of its positional parameters, in order. */
type Factory = (...values: never[]) => unknown;
type Entry = [positional: string[], factory: Factory];
const holdingParameters = ['holder', 'claim', 'acquisition'];
const factories: Record<string, Entry> = {
  'claim.entity': [['subject', 'predicate', 'objectId'], build.claim.entity],
  'claim.value': [['subject', 'predicate', 'value'], build.claim.value],
  'claim.unary': [['subject', 'predicate'], build.claim.unary],
  'claim.holds': [['holder', 'claimKey'], build.claim.holds],
  negate: [['claim'], build.negate],
  at: [['tick'], build.at],
  'by.human': [['actorId'], build.by.human],
  'by.model': [['model', 'promptHash'], build.by.model],
  'by.system': [['name'], build.by.system],
  'by.imported': [[], build.by.imported],
  witnessed: [['eventId'], build.witnessed],
  toldBy: [['sourceHolderId', 'intent', 'sourceRecordId'], build.toldBy],
  evidence: [['evidenceId'], build.evidence],
  inferred: [['fromRecordIds', 'rule'], build.inferred],
  authored: [[], build.authored],
  fact: [['claim'], build.fact],
  knows: [holdingParameters, build.knows],
  believes: [holdingParameters, build.believes],
  suspects: [holdingParameters, build.suspects],
  remembers: [holdingParameters, build.remembers],
};

function run(line: Line): unknown {
  const entry = factories[line.call];
  if (!entry) throw new Error(`builders.json calls '${line.call}', which this test does not know.`);
  const [positional, factory] = entry;
  const options = Object.fromEntries(Object.entries(line.args).filter(([key]) => !positional.includes(key)));
  const values = positional.map(name => line.args[name]);
  const call = factory as (...values: unknown[]) => unknown;
  return Object.keys(options).length > 0 ? call(...values, options) : call(...values);
}

test.each(lines.map((line, index) => [`${index + 1}: ${line.call}`, line] as const))('%s', (_name, line) => {
  const built = run(line);
  expect(built).toStrictEqual(line.expect);
  // Nothing is sent as `undefined`: the object is already its own JSON.
  expect(JSON.parse(JSON.stringify(built))).toStrictEqual(built);
});

test('every exported factory has a line in builders.json', () => {
  const exported = Object.entries(build).flatMap(([name, value]) =>
    typeof value === 'function' ? [name] : Object.keys(value as object).map(member => `${name}.${member}`));
  const called = new Set(lines.map(line => line.call));
  expect(exported.sort()).toEqual(Object.keys(factories).sort());
  expect(exported.filter(name => !called.has(name))).toEqual([]);
});

test('a factory never mutates what it is given', () => {
  const theft = build.claim.entity('bob', 'stole', 'necklace');
  const frozen = Object.freeze({ ...theft });
  expect(build.negate(frozen)).toEqual({ ...theft, polarity: false });
  expect(frozen.polarity).toBe(true);
});
