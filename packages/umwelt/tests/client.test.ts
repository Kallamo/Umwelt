// SPDX-License-Identifier: AGPL-3.0-only
import { mkdtempSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import type { ChildProcess } from 'node:child_process';
import { expect, test } from 'vitest';
import { createClient, UmweltError } from '../src/index.js';

test('automatic hello, concurrent correlation, errors, shutdown', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'umwelt-sdk-'));
  const client = createClient({ file: join(directory, 'test.umw'), create: true, binary: resolve(`target/debug/umwelt${process.platform === 'win32' ? '.exe' : ''}`) });
  try {
    const projects = await Promise.all(Array.from({ length: 20 }, (_, index) => client.projectCreate({ id: `p${index}`, name: `Project ${index}`, timescale: 'day' })));
    expect(projects.map(project => project.id)).toEqual(Array.from({ length: 20 }, (_, index) => `p${index}`));
    expect(projects.every(project => project.version === 0 && project.createdAt.length > 0)).toBe(true);
    await expect(client.projectGet({ projectId: 'missing' })).rejects.toMatchObject({ code: 'NOT_FOUND' });
    await expect(client.transport.request('holdings', { projectId: 'p0', holder: 'a', att: {} })).rejects.toMatchObject({ code: 'VALIDATION_FAILED', details: { unknownField: 'att' } });
  } finally { await client.close(); rmSync(directory, { recursive: true, force: true }); }
});

test('process death rejects all pending requests with a client-only error', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'umwelt-sdk-death-'));
  const client = createClient({ file: join(directory, 'test.umw'), create: true, binary: resolve(`target/debug/umwelt${process.platform === 'win32' ? '.exe' : ''}`) });
  try {
    await client.transport.ready;
    const pending = Array.from({ length: 100 }, () => client.projectList({}));
    (Reflect.get(client.transport, 'process') as ChildProcess).kill();
    const outcomes = await Promise.allSettled(pending);
    const failures = outcomes.filter(result => result.status === 'rejected');
    expect(failures.length).toBeGreaterThan(0);
    for (const failure of failures) {
      if (failure.status === 'rejected') {
        expect(failure.reason).toBeInstanceOf(UmweltError);
        expect(failure.reason.code).toBe('TRANSPORT_CLOSED');
      }
    }
  } finally { await client.close(); rmSync(directory, { recursive: true, force: true }); }
});
