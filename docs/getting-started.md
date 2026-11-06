# Getting started

This guide records one scene and asks who knows what happened in it, three
ways: from the command line, from TypeScript and from .NET. Each path takes a
few minutes and needs nothing else installed.

**The scene.** At a ball, Bob steals the duchess's necklace and Ana sees him
do it. Canon records the theft. Ana holds it because she witnessed it. Bob
holds nothing: being the thief is a fact about the world, and nobody, Bob
included, knows anything without a record of it.

The vocabulary used below (projects, entities, Facts, Holdings, events) is
defined in [concepts.md](concepts.md).

## Command line

Install the `umwelt` command from the
[latest release](https://github.com/Kallamo/Umwelt/releases), or build it with
Rust:

```bash
cargo install --git https://github.com/Kallamo/Umwelt umwelt-cli
```

Create a file, a project, the kinds of entities and the entities:

```bash
umwelt create story.umw
umwelt call story.umw project.create '{"id":"ball","name":"The Ball","timescale":"scene"}'
umwelt call story.umw entityType.create '{"projectId":"ball","id":"person","name":"Person","kind":"agent"}'
umwelt call story.umw entityType.create '{"projectId":"ball","id":"thing","name":"Thing","kind":"other"}'
umwelt call story.umw entity.create '{"projectId":"ball","id":"ana","typeId":"person","name":"Ana"}'
umwelt call story.umw entity.create '{"projectId":"ball","id":"bob","typeId":"person","name":"Bob"}'
umwelt call story.umw entity.create '{"projectId":"ball","id":"necklace","typeId":"thing","name":"The necklace"}'
```

`person` is an `agent`, so its entities can hold knowledge; `thing` is
`other`, so the necklace can be talked about but holds nothing. Projects are
`strict` by default, so predicates are declared before they are used:

```bash
umwelt call story.umw predicate.define '{"projectId":"ball","predicate":{"id":"stole","label":"stole","object":"entity","cardinality":"many"}}'
```

Record the scene in one commit: the event, the Fact, and Ana's Holding,
acquired by witnessing the event.

```bash
umwelt call story.umw commit '{
  "projectId": "ball", "attemptId": "scene-1",
  "event": {"id": "scene-1", "type": "scene", "title": "The ball", "position": {"tick": 1},
            "participants": ["ana", "bob"], "provenance": {"origin": {"kind": "human"}}},
  "records": [
    {"kind": "fact", "id": "theft", "position": {"tick": 1}, "provenance": {"origin": {"kind": "human"}},
     "claim": {"subject": "bob", "predicate": "stole", "object": {"kind": "entity", "id": "necklace"}, "polarity": true}},
    {"kind": "holding", "id": "ana-saw", "position": {"tick": 1}, "provenance": {"origin": {"kind": "human"}},
     "holder": "ana", "attitude": "knows", "confidence": 1, "accessibility": "available",
     "acquisition": {"kind": "witnessed", "eventId": "scene-1"},
     "claim": {"subject": "bob", "predicate": "stole", "object": {"kind": "entity", "id": "necklace"}, "polarity": true}}
  ]
}'
```

```json
{"v":1,"id":"call","ok":true,"result":{"eventId":"scene-1","recordIds":["theft","ana-saw"],"transitionIds":[],"version":1,"warnings":[]}}
```

Now ask who knows it:

```bash
umwelt call story.umw holds '{"projectId":"ball","holder":"ana","claim":{"subject":"bob","predicate":"stole","object":{"kind":"entity","id":"necklace"},"polarity":true}}'
```

```json
{"v":1,"id":"call","ok":true,"result":{"attitude":"knows","confidence":1.0,"holds":true,"polarity":true,"recordId":"ana-saw"}}
```

The same question for `bob` answers `{"holds":false}`, and so does the
question for Ana before the ball, with `"at":{"tick":0}`: reads never see
anything later than the position they ask about.

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
method:

```ts
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
using Kallamo.Umwelt;

using var engine = UmweltEngine.Create("story.umw");
const string projectId = "ball";
engine.ProjectCreate(new CreateProjectInput { Id = projectId, Name = "The Ball", Timescale = "scene" });
engine.EntityTypeCreate(new CreateEntityTypeInput { ProjectId = projectId, Id = "person", Name = "Person", Kind = EntityKind.Agent });
engine.EntityTypeCreate(new CreateEntityTypeInput { ProjectId = projectId, Id = "thing", Name = "Thing", Kind = EntityKind.Other });
engine.EntityCreate(new CreateEntityInput { ProjectId = projectId, Id = "ana", TypeId = "person", Name = "Ana" });
engine.EntityCreate(new CreateEntityInput { ProjectId = projectId, Id = "bob", TypeId = "person", Name = "Bob" });
engine.EntityCreate(new CreateEntityInput { ProjectId = projectId, Id = "necklace", TypeId = "thing", Name = "The duchess's necklace" });
engine.PredicateDefine(new DefinePredicateInput
{
    ProjectId = projectId,
    Predicate = new PredicateDef { Id = "stole", Label = "stole", Object = ObjectKind.Entity, Cardinality = Cardinality.Many },
});

var theft = new Claim
{
    Subject = "bob",
    Predicate = "stole",
    Object = new ClaimObject.Entity { Id = "necklace" },
    Polarity = true,
};
var provenance = new Provenance { Origin = new Origin.Human() };
engine.Commit(new CommitInput
{
    ProjectId = projectId,
    AttemptId = "scene-1",
    Event = new EventInput
    {
        Id = "scene-1",
        Type = "scene",
        Title = "The ball",
        Position = new Position { Tick = 1 },
        Participants = new() { "ana", "bob" },
        Provenance = provenance,
    },
    Records = new()
    {
        new RecordInput.Fact { Id = "theft", Position = new Position { Tick = 1 }, Provenance = provenance, Claim = theft },
        new RecordInput.Holding
        {
            Id = "ana-saw",
            Position = new Position { Tick = 1 },
            Provenance = provenance,
            Holder = "ana",
            Attitude = Attitude.Knows,
            Claim = theft,
            Acquisition = new Acquisition.Witnessed { EventId = "scene-1" },
            Confidence = 1.0,
            Accessibility = Accessibility.Available,
        },
    },
});

var ana = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "ana", Claim = theft });
var bob = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "bob", Claim = theft });
Console.WriteLine($"Ana knows: {ana.Holds}"); // Ana knows: True
Console.WriteLine($"Bob knows: {bob.Holds}"); // Bob knows: False
```

A complete project is in
[examples/csharp-minimal](../examples/csharp-minimal). Every operation is a
method named after it (`timeline.openDraft` is `TimelineOpenDraft`), and
errors are exceptions such as `NotFoundException` that carry the protocol
`code`. See the
[binding's guide](../bindings/csharp/Kallamo.Umwelt/README.md).

## Next steps

- Tell Bob what Ana saw with `transmit`, and follow the chain with `trace`.
- Put several witnesses in one scene with `witness`.
- Read Ana's whole point of view with `perspective`, or compare two holders
  with `compare`.
- Keep a day of play or a generated scene in a draft timeline until it is
  approved.

[concepts.md](concepts.md) explains each of these, [use-cases.md](use-cases.md)
shows how they fit tabletop campaigns, novels, games and AI contexts, and the
[specification](../spec/README.md) has every operation.
