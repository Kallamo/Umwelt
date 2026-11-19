// SPDX-License-Identifier: AGPL-3.0-only
// Writes the storage reference file of a published release.
//
//   node scripts/storage-fixture.mjs [--binary <path to umwelt>] [--force]
//
// Drives `umwelt serve` through a fixed scenario that touches every table,
// then records the answers of a fixed set of reads, and the writes whose
// retry must replay the stored outcome (`retries`). Writes
// fixtures/storage/v<storage>.umw and v<storage>.expected.json, where
// <storage> is the storage version the binary reports in `hello`.
//
// Run it once per published storage version, with the binary of that release.
// A published reference file is never regenerated: migrations are tested
// against it (see spec/versioning.md). The scenario grows with the engine, so
// it only runs with the release it was last extended for; each reference
// file records what its own release could write and read.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { existsSync, mkdirSync, mkdtempSync, copyFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const argument = name => {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
};
const binary = argument('--binary') ?? join(root, 'target', 'debug', process.platform === 'win32' ? 'umwelt.exe' : 'umwelt');
const force = process.argv.includes('--force');

const project = 'saga';
const provenance = { origin: { kind: 'human' } };
const claim = (subject, predicate, object, polarity = true) => ({ subject, predicate, object, polarity });
const entity = id => ({ kind: 'entity', id });
const value = v => ({ kind: 'value', value: v });
const holding = (id, holder, theClaim, extra = {}) => ({
  kind: 'holding', id, holder, attitude: 'knows', claim: theClaim,
  acquisition: { kind: 'authored' }, confidence: 1, accessibility: 'available', ...extra,
});
const fact = (id, theClaim) => ({ kind: 'fact', id, claim: theClaim });
const event = (id, type, title, tick, extra = {}) => ({ id, type, title, position: { tick }, provenance, ...extra });
const commit = (attemptId, theEvent, records, extra = {}) => ({ projectId: project, attemptId, event: theEvent, records, ...extra });

const letter = claim('ana', 'keeps', entity('letter'));
const meeting = claim('ana', 'met-at', value({ day: 1, place: 'mill' }));

// Every write must succeed. `embed` stands for the vectors of pending
// documents. A write marked `retry` is sent again by later engines, which must
// replay its outcome: their fingerprint of it matches the stored one.
const writes = [
  ['world.ensure', {
    project: { id: project, name: 'Saga', timescale: 'day' },
    entityTypes: [
      { id: 'person', name: 'Person', kind: 'agent' },
      { id: 'circle', name: 'Circle', kind: 'group' },
      { id: 'thing', name: 'Thing', kind: 'other' },
    ],
    entities: [
      { id: 'ana', typeId: 'person', name: 'Ana' },
      { id: 'bruno', typeId: 'person', name: 'Bruno' },
      { id: 'carla', typeId: 'person', name: 'Carla' },
      { id: 'circle', typeId: 'circle', name: 'The circle' },
      { id: 'letter', typeId: 'thing', name: 'The letter' },
    ],
    predicates: [
      { id: 'keeps', label: 'keeps', object: 'entity', cardinality: 'many' },
      { id: 'met-at', label: 'met at', object: 'value', cardinality: 'many' },
    ],
    axes: [{ id: 'reading', label: 'Reading order' }],
  }],
  ['entity.ensureBatch', { projectId: project, entities: [
    { externalRef: { system: 'importer', id: 'D-1' }, id: 'dario', typeId: 'person', name: 'Dario', aliases: ['D.'] },
  ] }],
  ['commit', commit('found', event('e-found', 'scene', 'The letter is found', 1, {
    participants: ['bruno', 'ana'], location: 'letter', on: { reading: 2 },
  }), [
    fact('f-letter', letter),
    fact('f-meeting', meeting),
    holding('h-ana', 'ana', letter, { text: 'Ana keeps the letter' }),
    holding('s-ana', 'ana', meeting, { text: 'Ana met someone at the mill', secrecy: { sharedWith: ['bruno'] } }),
  ])],
  // The same attempt again: replayed, not applied twice.
  ['commit', commit('found', event('e-found', 'scene', 'The letter is found', 1, {
    participants: ['bruno', 'ana'], location: 'letter', on: { reading: 2 },
  }), [
    fact('f-letter', letter),
    fact('f-meeting', meeting),
    holding('h-ana', 'ana', letter, { text: 'Ana keeps the letter' }),
    holding('s-ana', 'ana', meeting, { text: 'Ana met someone at the mill', secrecy: { sharedWith: ['bruno'] } }),
  ]), 'retry'],
  // Told outside the secret's circle: a breach.
  ['transmit', { projectId: project, attemptId: 'told-carla', position: { tick: 2 }, from: 'ana', to: 'carla',
    sourceRecordId: 's-ana', recordId: 'h-carla', confidence: 0.8, intent: 'deceptive', text: 'Carla heard about the mill' }, 'retry'],
  ['witness', { projectId: project, attemptId: 'gathering', event: event('e-gathering', 'scene', 'The circle gathers', 3, {
    participants: ['carla', 'bruno'], on: { reading: 1 },
  }), claims: [{ claim: claim('circle', 'keeps', entity('letter')), canon: true, factId: 'f-circle' }],
  witnesses: [{ holder: 'bruno', recordIds: ['w-bruno'] }, { holder: 'carla', attitude: 'suspects', confidence: 0.5, accessibility: 'available', recordIds: ['w-carla'] }] }, 'retry'],
  ['commit', commit('note', event('e-note', 'note', 'Sheet updated', 4, { participants: ['dario'] }), [
    { kind: 'awareness', id: 'aw-carla', holder: 'carla', target: 'dario' },
  ])],
  ['disposition.setBatch', { projectId: project, attemptId: 'trust', position: { tick: 4 }, provenance,
    dispositions: [{ id: 'd-trust', holder: 'ana', target: 'carla', dimension: 'trust', value: 0.3 }] }],
  ['holding.adjustBatch', { projectId: project, attemptId: 'doubt', position: { tick: 5 }, provenance,
    adjustments: [{ recordId: 'h-ana', newRecordId: 'h-ana-2', confidence: 0.5, attitude: 'believes', accessibility: 'available' }] }],
  ['transition.batch', { projectId: project, attemptId: 'forget', position: { tick: 6 }, status: 'forgotten', recordIds: ['w-carla'], provenance }],
  // Storage 6 (0.2.0): a Fact with text, and an event of another type whose
  // participants a narrowed read selects.
  ['commit', commit('burned', event('e-burned', 'report', 'The mill burns', 6, { participants: ['carla', 'dario'] }), [
    { ...fact('f-burned', claim('letter', 'met-at', value({ day: 6, place: 'mill' }))), text: 'The letter burned with the mill' },
  ]), 'retry'],
  ['embedding.defineSpace', { projectId: project, space: { id: 'words', dimensions: 3, model: 'hand-made' } }],
  ['embed', 'words'],
  // Drafts: one left open with a child, one promoted, one discarded, one purged.
  ['timeline.openDraft', { projectId: project, id: 'd-open' }],
  ['commit', commit('in-open', event('e-in-open', 'scene', 'A guess', 7, { participants: ['ana'] }),
    [holding('h-guess', 'bruno', claim('bruno', 'keeps', entity('letter')))], { timelineId: 'd-open' })],
  ['timeline.openDraft', { projectId: project, id: 'd-child', baseTimelineId: 'd-open' }],
  ['commit', commit('in-child', event('e-in-child', 'scene', 'A guess about a guess', 8, { participants: ['carla', 'ana'] }),
    [holding('h-child', 'carla', claim('carla', 'keeps', entity('letter')))], { timelineId: 'd-child' })],
  ['timeline.openDraft', { projectId: project, id: 'd-promoted' }],
  ['commit', commit('in-promoted', event('e-in-promoted', 'scene', 'Bruno leaves', 9, { participants: ['bruno'], on: { reading: 3 } }),
    [fact('f-left', claim('bruno', 'met-at', value('harbour')))], { timelineId: 'd-promoted' })],
  ['timeline.promote', { projectId: project, timelineId: 'd-promoted' }],
  ['timeline.openDraft', { projectId: project, id: 'd-discarded' }],
  ['commit', commit('in-discarded', event('e-in-discarded', 'scene', 'A dropped idea', 10, { participants: ['dario'] }),
    [fact('f-dropped', claim('dario', 'keeps', entity('letter')))], { timelineId: 'd-discarded' })],
  ['timeline.discard', { projectId: project, timelineId: 'd-discarded' }],
  ['timeline.openDraft', { projectId: project, id: 'd-purged' }],
  ['commit', commit('in-purged', event('e-in-purged', 'scene', 'Gone', 10),
    [fact('f-gone', claim('ana', 'met-at', value('nowhere')))], { timelineId: 'd-purged' })],
  ['timeline.discard', { projectId: project, timelineId: 'd-purged', purge: true }],
  ['propose', { projectId: project, event: event('e-proposed', 'scene', 'A proposal', 11, { participants: ['carla'] }),
    records: [fact('f-proposed', claim('carla', 'keeps', entity('letter')))] }],
];

const reads = [
  ['project.list', {}],
  ['entity.list', { projectId: project }],
  ['entity.resolve', { projectId: project, externalRef: { system: 'importer', id: 'D-1' } }],
  ['predicate.list', { projectId: project }],
  ['axis.list', { projectId: project }],
  ['embedding.listSpaces', { projectId: project }],
  ['timeline.list', { projectId: project }],
  ['timeline', { projectId: project }],
  ['timeline', { projectId: project, timelineId: 'd-child' }],
  ['timeline', { projectId: project, at: { axis: 'reading', tick: 2 } }],
  ['timeline', { projectId: project, at: { tick: 3, asOfVersion: 3 } }],
  ['perspective', { projectId: project, holder: 'ana' }],
  ['perspective', { projectId: project, holder: 'carla' }],
  ['perspective', { projectId: project, holder: 'carla', timelineId: 'd-child' }],
  ['perspective', { projectId: project, holder: 'bruno', at: { axis: 'reading', tick: 1 } }],
  ['holdings', { projectId: project, holder: 'carla' }],
  ['holds', { projectId: project, holder: 'ana', claim: letter }],
  ['whoHolds', { projectId: project, claim: meeting }],
  ['canon', { projectId: project }],
  ['compare', { projectId: project, a: 'ana', b: 'carla' }],
  ['contradictions', { projectId: project, holder: 'carla' }],
  ['trace', { projectId: project, recordId: 'h-carla' }],
  ['record', { projectId: project, recordId: 'h-ana' }],
  ['dispositions', { projectId: project, holder: 'ana' }],
  ['breaches', { projectId: project }],
  ['beliefsAbout', { projectId: project, holder: 'carla', about: 'ana' }],
  ['entity.at', { projectId: project, entityId: 'dario' }],
  ['changes', { projectId: project }],
  ['search', { projectId: project, text: 'letter' }],
  ['search', { projectId: project, text: 'mill', filter: { holder: 'carla' } }],
  ['search', { projectId: project, semantic: { spaceId: 'words', vector: [1, 0, 0] } }],
  ['search', { projectId: project, text: 'circle', semantic: { spaceId: 'words', vector: [0, 1, 0] } }],
  ['embedding.pending', { projectId: project, spaceId: 'words' }],
  ['proposal.list', { projectId: project }],
  // Reads added with storage 6 (0.2.0).
  ['timeline', { projectId: project, participant: 'carla' }],
  ['timeline', { projectId: project, participant: 'ana', eventTypes: ['scene'], range: { from: 1, to: 9 } }],
  ['timeline', { projectId: project, timelineId: 'd-child', participant: 'ana' }],
  ['timeline', { projectId: project, at: { axis: 'reading' }, participant: 'bruno', range: { from: 1, to: 2 } }],
  ['search', { projectId: project, text: 'burned' }],
  ['search', { projectId: project, text: 'letter', filter: { subject: 'letter' } }],
  ['search', { projectId: project, text: 'mill', filter: { holder: 'carla', eventTypes: ['umwelt:transmit'] }, range: { from: 2, to: 2 } }],
];

class Server {
  constructor(file) {
    this.child = spawn(binary, ['serve', file, '--create'], { stdio: ['pipe', 'pipe', 'inherit'] });
    this.waiting = [];
    this.next = 0;
    createInterface({ input: this.child.stdout }).on('line', line => this.waiting.shift()(JSON.parse(line)));
  }
  call(op, payload) {
    const id = String(++this.next);
    return new Promise(done => {
      this.waiting.push(done);
      this.child.stdin.write(`${JSON.stringify({ v: 1, id, op, payload })}\n`);
    });
  }
  async ok(op, payload) {
    const response = await this.call(op, payload);
    if (!response.ok) throw new Error(`${op} failed: ${JSON.stringify(response.error)}`);
    return response.result;
  }
  close() {
    return new Promise(done => {
      this.child.on('exit', done);
      this.child.stdin.end();
    });
  }
}

// A deterministic vector per document, so the file is the same on every run.
const vectorFor = text => {
  const sums = [0, 0, 0];
  [...text].forEach((character, index) => { sums[index % 3] += character.charCodeAt(0); });
  return sums.map(sum => (sum % 97) / 97 + 0.01);
};

const scratch = mkdtempSync(join(tmpdir(), 'umwelt-storage-fixture-'));
const file = join(scratch, 'reference.umw');
const server = new Server(file);
const hello = await server.ok('hello', { protocol: { min: 1, max: 1 }, client: 'storage-fixture' });
const storage = hello.storage;
const target = join(root, 'fixtures', 'storage');
const umw = join(target, `v${storage}.umw`);
const expected = join(target, `v${storage}.expected.json`);
if (!force && (existsSync(umw) || existsSync(expected))) {
  await server.close();
  rmSync(scratch, { recursive: true, force: true });
  throw new Error(`fixtures/storage/v${storage}.* already exists. Published reference files are never regenerated.`);
}

const retries = [];
for (const [op, payload, mark] of writes) {
  if (op === 'embed') {
    const pending = await server.ok('embedding.pending', { projectId: project, spaceId: payload });
    const items = pending.items.map(item => ({ docType: item.docType, docId: item.docId, textHash: item.textHash, vector: vectorFor(item.text) }));
    await server.ok('embedding.put', { projectId: project, spaceId: payload, items });
  } else {
    const result = await server.ok(op, payload);
    if (mark === 'retry') retries.push({ op, payload, result });
  }
}
const recorded = [];
for (const [op, payload] of reads) {
  recorded.push({ op, payload, result: await server.ok(op, payload) });
}
await server.close();

const leftovers = readdirSync(scratch).filter(name => name !== 'reference.umw' && !name.endsWith('.lock'));
if (leftovers.length > 0) {
  throw new Error(`The engine left ${leftovers.join(', ')} next to the file; the reference must be a single file.`);
}
mkdirSync(target, { recursive: true });
copyFileSync(file, umw);
writeFileSync(expected, `${JSON.stringify({
  engine: hello.engine,
  storage,
  note: 'Answers of the reads below, recorded by the release that wrote the file. Never edit.',
  reads: recorded,
  retries,
}, null, 2)}\n`);
rmSync(scratch, { recursive: true, force: true });
console.log(`wrote fixtures/storage/v${storage}.umw and v${storage}.expected.json (engine ${hello.engine})`);
