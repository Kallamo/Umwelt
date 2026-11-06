// SPDX-License-Identifier: AGPL-3.0-only
// The getting-started scene: Ana sees Bob steal the necklace. Canon records
// the theft; only Ana holds it.
import { createClient } from '@kallamo/umwelt';
import type { Claim, Provenance } from '@kallamo/umwelt';

const client = createClient({ file: 'story.umw', create: true });
try {
  const projectId = 'ball';
  await client.projectCreate({ id: projectId, name: 'The Ball', timescale: 'scene' });
  await client.entityTypeCreate({ projectId, id: 'person', name: 'Person', kind: 'agent' });
  await client.entityTypeCreate({ projectId, id: 'thing', name: 'Thing', kind: 'other' });
  await client.entityCreate({ projectId, id: 'ana', typeId: 'person', name: 'Ana' });
  await client.entityCreate({ projectId, id: 'bob', typeId: 'person', name: 'Bob' });
  await client.entityCreate({ projectId, id: 'necklace', typeId: 'thing', name: "The duchess's necklace" });
  await client.predicateDefine({
    projectId,
    predicate: { id: 'stole', label: 'stole', object: 'entity', cardinality: 'many' },
  });

  const theft: Claim = {
    subject: 'bob',
    predicate: 'stole',
    object: { kind: 'entity', id: 'necklace' },
    polarity: true,
  };
  const provenance: Provenance = { origin: { kind: 'human' } };
  await client.commit({
    projectId,
    attemptId: 'scene-1',
    event: {
      id: 'scene-1',
      type: 'scene',
      title: 'The ball',
      position: { tick: 1 },
      participants: ['ana', 'bob'],
      provenance,
    },
    records: [
      { kind: 'fact', id: 'theft', position: { tick: 1 }, provenance, claim: theft },
      {
        kind: 'holding',
        id: 'ana-saw',
        position: { tick: 1 },
        provenance,
        holder: 'ana',
        attitude: 'knows',
        claim: theft,
        acquisition: { kind: 'witnessed', eventId: 'scene-1' },
        confidence: 1,
        accessibility: 'available',
      },
    ],
  });

  const ana = await client.holds({ projectId, holder: 'ana', claim: theft });
  const bob = await client.holds({ projectId, holder: 'bob', claim: theft });
  console.log(`Ana knows: ${ana.holds}`); // Ana knows: true
  console.log(`Bob knows: ${bob.holds}`); // Bob knows: false
} finally {
  await client.close();
}
