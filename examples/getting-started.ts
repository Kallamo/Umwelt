// SPDX-License-Identifier: AGPL-3.0-only
// The getting-started scene: Ana sees Bob steal the necklace. Canon records
// the theft; Ana and Bob know it, and the duchess, though present, does not.
// The next morning she suspects the wrong person.
import { at, authored, by, claim, createClient, suspects } from '@kallamo/umwelt';

const client = createClient({ file: 'story.umw', create: true });
try {
  const projectId = 'ball';
  // The world, declared once. It creates what is missing and changes nothing
  // that is already there, so it can run every time the file is opened.
  await client.worldEnsure({
    project: { id: projectId, name: 'The Ball', timescale: 'scene' },
    entityTypes: [
      { id: 'person', name: 'Person', kind: 'agent' },
      { id: 'thing', name: 'Thing', kind: 'other' },
    ],
    entities: [
      { id: 'ana', typeId: 'person', name: 'Ana' },
      { id: 'bob', typeId: 'person', name: 'Bob' },
      { id: 'duchess', typeId: 'person', name: 'The duchess' },
      { id: 'necklace', typeId: 'thing', name: "The duchess's necklace" },
    ],
    predicates: [{ id: 'stole', label: 'stole', object: 'entity', cardinality: 'many' }],
  });

  // The scene: one event, the Fact (canon) and what each witness saw.
  const theft = claim.entity('bob', 'stole', 'necklace');
  await client.witness({
    projectId,
    attemptId: 'scene-1',
    event: {
      id: 'scene-1',
      type: 'scene',
      title: 'The ball',
      position: at(1),
      participants: ['ana', 'bob', 'duchess'],
      provenance: by.human(),
    },
    claims: [{ claim: theft, canon: true }],
    witnesses: [{ holder: 'ana' }, { holder: 'bob' }],
  });

  const ana = await client.holds({ projectId, holder: 'ana', claim: theft });
  const bob = await client.holds({ projectId, holder: 'bob', claim: theft });
  const duchess = await client.holds({ projectId, holder: 'duchess', claim: theft });
  // `holds` is also true for someone who believes the opposite; `holdsAs` tells them apart.
  console.log(`Ana knows: ${ana.holdsAs === 'same'}`); // Ana knows: true
  console.log(`Bob knows: ${bob.holdsAs === 'same'}`); // Bob knows: true
  console.log(`The duchess knows: ${duchess.holdsAs === 'same'}`); // The duchess knows: false

  // The morning after: nobody saw anything and nobody told her, so this is a
  // record written directly. It takes its position and provenance from the event.
  const accusation = claim.entity('ana', 'stole', 'necklace');
  await client.commit({
    projectId,
    attemptId: 'morning',
    event: { type: 'scene', title: 'The morning after', position: at(2), provenance: by.human() },
    records: [suspects('duchess', accusation, authored(), { confidence: 0.6 })],
  });

  const suspicion = await client.holds({ projectId, holder: 'duchess', claim: accusation });
  console.log(`The duchess ${suspicion.attitude} Ana: ${suspicion.confidence}`); // The duchess suspects Ana: 0.6
} finally {
  await client.close();
}
