// SPDX-License-Identifier: AGPL-3.0-only
import { expect, test } from 'vitest';
import { resolveEngineBinary } from '../src/index.js';

test('UMWELT_BINARY overrides the bundled engine', () => {
  expect(resolveEngineBinary({ UMWELT_BINARY: '/opt/umwelt' }, 'linux', 'x64')).toBe('/opt/umwelt');
});

test('without a bundled engine for the platform, umwelt comes from the PATH', () => {
  expect(resolveEngineBinary({}, 'aix', 'ppc64')).toBe('umwelt');
});
