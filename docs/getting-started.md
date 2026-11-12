# Getting started

This guide records one scene and asks who knows what happened in it, three
ways: from the command line, from TypeScript and from .NET. Each path takes a
few minutes and needs nothing else installed.

**The scene.** At a ball, Bob steals the duchess's necklace and Ana sees him
do it. Canon records the theft. Ana knows because she witnessed it, and Bob
knows because he did it. The duchess was at the ball too, but she knows
nothing: being present grants no knowledge. Nobody knows anything without a
record of it, and Umwelt never infers one: the application records what each
character knows, here including Bob's own knowledge of what he did.

The vocabulary used below (projects, entities, Facts, Holdings, events) is
defined in [concepts.md](concepts.md).

## Command line

Install the `umwelt` command from the
[latest release](https://github.com/Kallamo/Umwelt/releases), or build it with
Rust:

```bash
cargo install --git https://github.com/Kallamo/Umwelt umwelt-cli
```

Create a file, then declare the world in one call: a project, the kinds of
entities, the entities and the predicates.

```bash
umwelt create story.umw
umwelt call story.umw world.ensure '{
  "project": {"id": "ball", "name": "The Ball", "timescale": "scene"},
  "entityTypes": [{"id": "person", "name": "Person", "kind": "agent"},
                  {"id": "thing", "name": "Thing", "kind": "other"}],
  "entities": [{"id": "ana", "typeId": "person", "name": "Ana"},
               {"id": "bob", "typeId": "person", "name": "Bob"},
               {"id": "duchess", "typeId": "person", "name": "The duchess"},
               {"id": "necklace", "typeId": "thing", "name": "The necklace"}],
  "predicates": [{"id": "stole", "label": "stole", "object": "entity", "cardinality": "many"}]
}'
```

`person` is an `agent`, so its entities can hold knowledge; `thing` is
`other`, so the necklace can be talked about but holds nothing. Projects are
`strict` by default, so predicates are declared before they are used.

The manifest creates what is missing and leaves the rest as it is: sending it
again changes nothing, so an application can send it every time it opens the
file. Each of these can also be created on its own (`project.create`,
`entityType.create`, `entity.create`, `predicate.define`).

Record the scene with `witness`: one event, the claim as canon (a Fact), and
a Holding for each witness, acquired at that event. The duchess is a
participant, and gets no Holding.

```bash
umwelt call story.umw witness '{
  "projectId": "ball", "attemptId": "scene-1",
  "event": {"id": "scene-1", "type": "scene", "title": "The ball", "position": {"tick": 1},
            "participants": ["ana", "bob", "duchess"], "provenance": {"origin": {"kind": "human"}}},
  "claims": [{"canon": true, "factId": "theft",
              "claim": {"subject": "bob", "predicate": "stole", "object": {"kind": "entity", "id": "necklace"}, "polarity": true}}],
  "witnesses": [{"holder": "ana", "recordIds": ["ana-saw"]}, {"holder": "bob", "recordIds": ["bob-did"]}]
}'
```

```json
{"v":1,"id":"call","ok":true,"result":{"eventId":"scene-1","recordIds":["theft","ana-saw","bob-did"],"transitionIds":[],"version":1,"warnings":[]}}
```

A witness knows what they saw (attitude `knows`, confidence 1) unless the
request says otherwise. The ids are optional: left out, the engine generates
them.

Now ask who knows it:

```bash
umwelt call story.umw holds '{"projectId":"ball","holder":"ana","claim":{"subject":"bob","predicate":"stole","object":{"kind":"entity","id":"necklace"},"polarity":true}}'
```

```json
{"v":1,"id":"call","ok":true,"result":{"attitude":"knows","confidence":1.0,"holds":true,"holdsAs":"same","polarity":true,"recordId":"ana-saw"}}
```

`holdsAs` is `same`: Ana holds the claim as it was asked. Someone who believed
Bob did *not* steal it would also answer `holds: true`, with
`holdsAs: "opposite"`, so a check for "does she know?" reads `holdsAs`.

The same question for `bob` answers `true` with his own record, `bob-did`.
For `duchess` it answers `{"holds":false}`: she was there, and knows nothing.
So does the question for Ana before the ball, with `"at":{"tick":0}`: reads
never see anything later than the position they ask about.

`umwelt call` opens the file, answers one request and exits. An application
keeps one process open instead, with `umwelt serve story.umw`, which reads one
JSON request per line on standard input and answers on standard output (see
[the protocol](../spec/protocol.md)). The client libraries below do this for
you.

## TypeScript

```bash
npm install @kallamo/umwelt
```

The package includes the `umwelt` engine for Windows, Linux and macOS. The
client starts it, performs the handshake and gives every operation a typed
method. `claim`, `at`, `by` and the other small functions it exports build the
objects of a request:

```ts
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
```

This program is [examples/getting-started.ts](../examples/getting-started.ts).
Failed requests reject with an `UmweltError` that carries the protocol `code`.
The client's own guide is
[packages/umwelt/README.md](../packages/umwelt/README.md).

## .NET

```bash
dotnet add package Kallamo.Umwelt
```

The binding runs the engine inside your process through its native library,
included in the package for Windows, Linux and macOS. It targets .NET 6 and
later.

```csharp
using System.Globalization;
using Kallamo.Umwelt;

using var engine = UmweltEngine.Create("story.umw");
const string projectId = "ball";
// The world, declared once. It creates what is missing and changes nothing
// that is already there, so it can run every time the file is opened.
engine.WorldEnsure(new EnsureWorldInput
{
    Project = new WorldProject { Id = projectId, Name = "The Ball", Timescale = "scene" },
    EntityTypes = new()
    {
        new WorldEntityType { Id = "person", Name = "Person", Kind = EntityKind.Agent },
        new WorldEntityType { Id = "thing", Name = "Thing", Kind = EntityKind.Other },
    },
    Entities = new()
    {
        new WorldEntity { Id = "ana", TypeId = "person", Name = "Ana" },
        new WorldEntity { Id = "bob", TypeId = "person", Name = "Bob" },
        new WorldEntity { Id = "duchess", TypeId = "person", Name = "The duchess" },
        new WorldEntity { Id = "necklace", TypeId = "thing", Name = "The duchess's necklace" },
    },
    Predicates = new() { new PredicateDef { Id = "stole", Label = "stole", Object = ObjectKind.Entity, Cardinality = Cardinality.Many } },
});

// The scene: one event, the Fact (canon) and what each witness saw.
var theft = Claim.Entity("bob", "stole", "necklace");
engine.Witness(new WitnessInput
{
    ProjectId = projectId,
    AttemptId = "scene-1",
    Event = new EventInput
    {
        Id = "scene-1",
        Type = "scene",
        Title = "The ball",
        Position = Position.At(1),
        Participants = new() { "ana", "bob", "duchess" },
        Provenance = Provenance.Human(),
    },
    Claims = new() { new WitnessedClaim { Claim = theft, Canon = true } },
    Witnesses = new() { new Witness { Holder = "ana" }, new Witness { Holder = "bob" } },
});

var ana = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "ana", Claim = theft });
var bob = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "bob", Claim = theft });
var duchess = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "duchess", Claim = theft });
// Holds is also true for someone who believes the opposite; HoldsAs tells them apart.
Console.WriteLine($"Ana knows: {ana.HoldsAs == HoldsAs.Same}"); // Ana knows: True
Console.WriteLine($"Bob knows: {bob.HoldsAs == HoldsAs.Same}"); // Bob knows: True
Console.WriteLine($"The duchess knows: {duchess.HoldsAs == HoldsAs.Same}"); // The duchess knows: False

// The morning after: nobody saw anything and nobody told her, so this is a
// record written directly. It takes its position and provenance from the event.
var accusation = Claim.Entity("ana", "stole", "necklace");
engine.Commit(new CommitInput
{
    ProjectId = projectId,
    AttemptId = "morning",
    Event = new EventInput { Type = "scene", Title = "The morning after", Position = Position.At(2), Provenance = Provenance.Human() },
    Records = new() { Records.Suspects("duchess", accusation, Acquired.Authored(), confidence: 0.6) },
});

var suspicion = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "duchess", Claim = accusation });
var confidence = suspicion.Confidence?.ToString(CultureInfo.InvariantCulture);
Console.WriteLine($"The duchess {suspicion.Attitude?.ToString().ToLowerInvariant()} Ana: {confidence}"); // The duchess suspects Ana: 0.6
```

A complete project is in
[examples/csharp-minimal](../examples/csharp-minimal). Every operation is a
method named after it (`timeline.openDraft` is `TimelineOpenDraft`), and
errors are exceptions such as `NotFoundException` that carry the protocol
`code`. See the
[binding's guide](../bindings/csharp/Kallamo.Umwelt/README.md).

## Which write to use

Both programs end with a second write. The morning after the ball the duchess
finds the necklace gone and suspects Ana. Nobody saw anything and nobody told
her, so neither `witness` nor `transmit` fits: the application writes the
record itself, with `commit`.

| Operation | Use it when |
| --- | --- |
| `witness` | Holders were at an event and saw the same claims. |
| `transmit` | One holder tells another something they hold. |
| `commit` | Anything else: a belief with no in-world source, an inference, a Fact alone, Awareness, Dispositions, transitions. |

From the command line, the same commit is:

```bash
umwelt call story.umw commit '{
  "projectId": "ball", "attemptId": "morning",
  "event": {"type": "scene", "title": "The morning after", "position": {"tick": 2},
            "provenance": {"origin": {"kind": "human"}}},
  "records": [
    {"kind": "holding", "id": "suspicion", "holder": "duchess", "attitude": "suspects", "confidence": 0.6,
     "acquisition": {"kind": "authored"},
     "claim": {"subject": "ana", "predicate": "stole", "object": {"kind": "entity", "id": "necklace"}, "polarity": true}}
  ]
}'
```

A record says who holds it, with which attitude and how they came to hold it;
those are never assumed. What it leaves out takes a fixed default:

| Left out | Default |
| --- | --- |
| `position`, `provenance` (records and transitions) | Those of the commit's event. |
| `confidence` (Holding) | `1` |
| `accessibility` (Holding) | `available` |

Reading the record back shows everything that was stored:

```bash
umwelt call story.umw record '{"projectId":"ball","recordId":"suspicion"}'
```

```json
{"v":1,"id":"call","ok":true,"result":{"accessibility":"available","acquisition":{"kind":"authored"},"attitude":"suspects","claim":{"object":{"id":"necklace","kind":"entity"},"polarity":true,"predicate":"stole","qualifiers":{},"subject":"ana"},"confidence":0.6,"eventId":"...","holder":"duchess","id":"suspicion","kind":"holding","position":{"tick":2},"projectId":"ball","provenance":{"origin":{"kind":"human"}},"recordedAt":2,"timelineId":"main"}}
```

The client libraries build these objects with small functions. Each one is
pure and returns the plain object of the protocol, so it can be mixed freely
with hand-written ones:

| | TypeScript | .NET |
| --- | --- | --- |
| Claims | `claim.entity`, `claim.value`, `claim.unary`, `claim.holds`, `negate` | `Claim.Entity`, `Claim.Value`, `Claim.Unary`, `Claim.Holds`, `claim.Negated()` |
| Positions | `at(tick)` | `Position.At(tick)` |
| Provenance | `by.human`, `by.model`, `by.system`, `by.imported` | `Provenance.Human`, `.Model`, `.System`, `.Imported` |
| Acquisitions | `witnessed`, `toldBy`, `evidence`, `inferred`, `authored` | `Acquired.Witnessed`, `.ToldBy`, `.Evidence`, `.Inferred`, `.Authored` |
| Records | `fact`, `knows`, `believes`, `suspects`, `remembers` | `Records.Fact`, `.Knows`, `.Believes`, `.Suspects`, `.Remembers` |

## Next steps

- Have Bob lie to the duchess with `transmit` (intent `deceptive`): she ends up
  believing a servant stole it, a belief Umwelt keeps and reports against
  canon. Follow the chain with `trace`.
- Read Ana's whole point of view with `perspective`, or compare two holders
  with `compare`.
- Keep a day of play or a generated scene in a draft timeline until it is
  approved.

[concepts.md](concepts.md) explains each of these, [use-cases.md](use-cases.md)
shows how they fit tabletop campaigns, novels, games and AI contexts, and the
[specification](../spec/README.md) has every operation.
