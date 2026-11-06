// SPDX-License-Identifier: AGPL-3.0-only
import { mkdtempSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { expect, test } from 'vitest';
import { createClient } from '../src/index.js';
import type { EmbeddingClient, Embedder } from '../src/index.js';

const binary = resolve(`target/debug/umwelt${process.platform === 'win32' ? '.exe' : ''}`);

/** A deterministic stand-in for a model: one dimension per topic. */
function fakeEmbedder(calls: string[][], autoEmbed = false): Embedder {
  return {
    spaceId: 'topics',
    batchSize: 2,
    autoEmbed,
    embed: texts => {
      calls.push(texts);
      return texts.map(text => [
        /jewel|necklace|stole/.test(text) ? 1 : 0.01,
        /rain|storm/.test(text) ? 1 : 0.01,
        /bread|market/.test(text) ? 1 : 0.01,
      ]);
    },
  };
}

async function setUp(client: EmbeddingClient): Promise<void> {
  await client.projectCreate({ id: 'p', name: 'Town', timescale: 'day', vocabularyMode: 'open' });
  await client.entityTypeCreate({ projectId: 'p', id: 'agent', name: 'Agent', kind: 'agent' });
  await client.entityCreate({ projectId: 'p', id: 'ana', typeId: 'agent', name: 'Ana' });
  await client.embeddingDefineSpace({ projectId: 'p', space: { id: 'topics', dimensions: 3, model: 'fake' } });
}

const note = (id: string, text: string) => ({
  kind: 'holding' as const, id, position: { tick: 1 }, provenance: { origin: { kind: 'human' as const } },
  holder: 'ana', attitude: 'believes' as const, confidence: 1, accessibility: 'available' as const,
  acquisition: { kind: 'authored' as const }, text,
  claim: { subject: 'ana', predicate: 'notes', object: { kind: 'value' as const, value: id }, polarity: true },
});

const commit = (attemptId: string, records: ReturnType<typeof note>[]) => ({
  projectId: 'p', attemptId,
  event: { id: `e-${attemptId}`, type: 'umwelt:test-day', title: 'Day', position: { tick: 1 }, provenance: { origin: { kind: 'human' as const } } },
  records,
});

test('embedPending fills vectors in batches and semanticSearch finds by meaning', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'umwelt-embed-'));
  const calls: string[][] = [];
  const client = createClient({ file: join(directory, 'test.umw'), create: true, binary, embedder: fakeEmbedder(calls) });
  try {
    await setUp(client);
    await client.commit(commit('day-1', [
      note('necklace', 'someone took the jewel'),
      note('weather', 'a storm all night'),
      note('market', 'bread is expensive'),
    ]));
    const filled = await client.embedPending({ projectId: 'p' });
    expect(filled.embedded).toBe(3);
    expect(calls.map(batch => batch.length)).toEqual([2, 1]);
    expect((await client.embedPending({ projectId: 'p' })).embedded).toBe(0);

    const found = await client.semanticSearch({ projectId: 'p', text: 'who stole the necklace?', mode: 'semantic', filter: { holder: 'ana' } });
    expect(found.items[0]).toMatchObject({ docId: 'necklace', match: 'semantic' });
    const hybrid = await client.semanticSearch({ projectId: 'p', text: 'bread', filter: { holder: 'ana' } });
    expect(hybrid.items[0]).toMatchObject({ docId: 'market', match: 'both' });
  } finally { await client.close(); rmSync(directory, { recursive: true, force: true }); }
});

test('autoEmbed embeds each write, resuming after the last document seen', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'umwelt-auto-'));
  const calls: string[][] = [];
  const client = createClient({ file: join(directory, 'test.umw'), create: true, binary, embedder: fakeEmbedder(calls, true) });
  try {
    await setUp(client);
    await client.commit(commit('day-1', [note('necklace', 'someone took the jewel')]));
    await client.commit(commit('day-2', [note('weather', 'a storm all night')]));
    expect(calls).toEqual([['someone took the jewel'], ['a storm all night']]);
    const pending = await client.embeddingPending({ projectId: 'p', spaceId: 'topics' });
    expect(pending.items).toEqual([]);
  } finally { await client.close(); rmSync(directory, { recursive: true, force: true }); }
});

test('the helpers need an embedder', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'umwelt-none-'));
  const client = createClient({ file: join(directory, 'test.umw'), create: true, binary });
  try {
    await expect(client.embedPending({ projectId: 'p' })).rejects.toThrow('No embedder');
  } finally { await client.close(); rmSync(directory, { recursive: true, force: true }); }
});
