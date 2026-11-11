<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Umwelt JSON protocol v1

Umwelt protocol v1 is newline-delimited JSON over standard input and standard output. Each request and response occupies exactly one line. A server writes only response envelopes to stdout; diagnostics go to stderr.

## Envelopes

```json
{"v":1,"id":"r1","op":"project.get","payload":{"projectId":"p1"}}
{"v":1,"id":"r1","ok":true,"result":{"id":"p1"}}
{"v":1,"id":"r1","ok":false,"error":{"code":"NOT_FOUND","message":"Project 'p1' does not exist."}}
```

`id` is copied from the request. For invalid JSON, the dispatcher recovers a string `id` when it can be read from the incomplete input; otherwise the response contains `"id":null`. One malformed request never terminates the dispatcher. Error codes are defined in [errors.md](errors.md).

## Handshake

The first valid request must be:

```json
{"v":1,"id":"r1","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"client-name/version"}}
```

The server selects protocol 1 and returns the contract version, the storage version (see [storage.md](storage.md) and [versioning.md](versioning.md)), the engine version, and every supported operation as a `command` or `query` capability. A different first operation returns `HANDSHAKE_REQUIRED`. A range that excludes 1 returns `UNSUPPORTED_PROTOCOL` with `{"supported":{"min":1,"max":1}}` in `details`.

## Operations

Commands are `project.create`, `entityType.create`, `entity.create`, `entity.update`, `entity.archive`, `predicate.define`, `axis.define`, `embedding.defineSpace`, `embedding.dropSpace`, `embedding.put`, `embedding.delete`, `evidence.register`, `commit`, `commitBatch`, `propose`, `proposal.review`, `proposal.apply`, `proposal.reject`, `transmit`, `transmitBatch`, `witness`, `holding.adjustBatch`, `transition.batch`, `disposition.setBatch`, `entity.ensureBatch`, `proposal.rebase`, `timeline.openDraft`, `timeline.promote`, `timeline.rebase`, `timeline.discard`, `timeline.purge`, `world.ensure`, and `shutdown`.

Queries are `project.get`, `project.list`, `entityType.list`, `entity.get`, `entity.list`, `predicate.list`, `axis.list`, `embedding.listSpaces`, `embedding.pending`, `evidence.get`, `proposal.get`, `proposal.list`, `holds`, `holdings`, `whoHolds`, `canon`, `perspective`, `compare`, `contradictions`, `trace`, `timeline`, `record`, `changes`, `search`, `timeline.get`, `timeline.list`, `entity.resolve`, `entity.resolveBatch`, `entity.at`, `entity.asKnownBy`, `groupCoverage`, `beliefsAbout`, `breaches`, `check.utterance`, `context.admit`, `claim.key`, and `dispositions`.

The operations for external entities, witnesses, groups, batch maintenance, names over time, second-order knowledge, secrets, continuity checks, context admission and rebase are specified in [knowledge-operations.md](knowledge-operations.md). A successful commit may carry `warnings`; `UNKNOWN_PREDICATE` (open vocabulary) and `SECRET_BREACH` (a secret told outside its circle) never block a write.

Operation payloads use the public v1 contract and reject unknown fields. Read operations use the public `At` and `Page` objects. `commitBatch` receives `{ "commits": [...] }`; `transmitBatch` receives `{ "transmits": [...] }`.

## Timelines

Every write and read operation accepts an optional `timelineId` (default `main`). Writes go to that timeline; reads see it through the visibility rule of [timelines.md](timelines.md): a draft sees its own rows plus its ancestors' rows up to the versions its chain was opened from. A draft branches from main or from another active draft, at most four deep. All commits of one `commitBatch` or `transmitBatch` must target the same timeline. `changes` is global by default and takes `timelineId` as a filter; every change item carries its `timelineId`.

`timeline.openDraft`, `timeline.promote`, `timeline.rebase`, `timeline.discard` (optionally with `purge: true`), `timeline.purge`, `timeline.get` and `timeline.list` manage drafts. Promote moves a draft into its parent and fails with `VERSION_CONFLICT` and `details: { baseVersion, parentTimelineId, parentHeadVersion }` (plus `mainHeadVersion` when the parent is main) when the parent changed since the draft was opened; the draft is left intact. Writing to a draft that is not active, reading a promoted draft (`details.promotedInto`), purging main or a promoted draft, and discarding, purging or rebasing a draft that other drafts depend on (`details.descendants`) return `INVALID_STATE`. Reading a purged draft returns `NOT_FOUND`. `Project.version` is main's head version.

Query payloads are strict too: an unknown top-level key returns
`VALIDATION_FAILED`, with `details.unknownField` identifying the key. For example,
`att` is not an alias for `at` in `holds` or `holdings`.

Creation operations never accept fields controlled by the engine:

- `project.create`: `{ id?, name, description?, timescale, vocabularyMode? }`. The default vocabulary mode is `strict`; the engine supplies a missing ID, `createdAt`, and version 0.
- `entityType.create`: `{ projectId, id?, name, kind, fields? }`. `fields` defaults to an empty array.
- `entity.create`: `{ projectId, id?, typeId, name, aliases?, fields?, externalRef?, createdAt? }`. Collections default empty, `createdAt` defaults to `{ "tick": 0 }`, and the engine sets `archived` to false.
- `entity.update`: `{ projectId, entityId, name?, aliases?, fields?, externalRef? }`. Only present mutable fields change. `projectId`, `typeId`, and `archived` cannot be changed, and fields are revalidated against the existing entity type.
- `predicate.define`: `{ projectId, predicate }`. Predicate IDs beginning with `umwelt:` are reserved.
- `embedding.defineSpace`: `{ projectId, space: { id, dimensions, model } }`. Space IDs beginning with `umwelt:` are reserved; see [search.md](search.md).
- `axis.define`: `{ projectId, axis: { id, label } }`. The id `tick` and IDs beginning with `umwelt:` are reserved; see [queries.md](queries.md#time-axes).

`project.create`, `entityType.create`, `entity.create` and `predicate.define`
never overwrite: an id that already exists, or an `externalRef` another entity
of the project already has, fails with `CONFLICT` and changes nothing. To
create only what is missing, use `world.ensure` or `entity.ensureBatch`.

### Write defaults

A commit (`commit`, `commitBatch`, `propose`) may leave out fields that repeat
its event or have one usual value. The defaults are fixed and never depend on
the data:

| Field | Left out |
| --- | --- |
| `position` of a record or a transition | The position of the commit's event. |
| `provenance` of a record or a transition | The provenance of the commit's event. |
| `confidence` of a Holding | `1` |
| `accessibility` of a Holding | `available` |

`holder`, `attitude`, `acquisition` and `claim` (with its `polarity`) have no
default: who holds something, with which attitude and how they came to hold it
is always stated by the client. `witness` and `transmit` document their own
defaults in [knowledge-operations.md](knowledge-operations.md).

Defaults are filled before the request is fingerprinted and validated, and
the stored record is always complete. A retry that spells out a field the
first request left to its default is therefore the same attempt and returns
the original outcome; one that gives it another value is a `CONFLICT`. A
`propose` stores and returns the filled records, so a reviewer sees what would
be written. An inherited position goes through the same checks as a written
one: a transition still cannot precede the record it targets.

### A batch of transmits

`transmitBatch` receives `{ "shared"?: {...}, "transmits": [...] }`. What the
transmits have in common is written once in `shared` (`projectId`,
`timelineId`, `position`, `on`, `from`, `to`, `sourceRecordId`, `intent`,
`attitude`, `confidence`), and each item writes what differs; a field given by
an item wins. `shared` cannot carry what identifies one transmit (`attemptId`,
`recordId`, `text`, `distortedClaim`).

After the two are joined every item must be complete: `intent` and
`confidence` are still required, they can just be stated once. The engine
supplies neither; only `attitude` has a default (`believes`). Items are
joined before they are fingerprinted, so a retry that writes a field in the
item instead of in `shared` is the same attempt. `transmit` takes one complete
transmit and has no `shared`.

### A world manifest

`world.ensure` declares what must exist:

```json
{
  "project": { "id": "ball", "name": "The Ball", "timescale": "scene" },
  "entityTypes": [{ "id": "person", "name": "Person", "kind": "agent" }],
  "entities": [{ "id": "ana", "typeId": "person", "name": "Ana" }],
  "predicates": [{ "id": "stole", "label": "stole", "object": "entity", "cardinality": "many" }],
  "axes": [{ "id": "reading", "label": "Reading order" }]
}
```

Every id is required, and every section but `project` is optional.

- **Atomic.** Everything missing is created in one transaction, or nothing.
- **Repeatable.** What already exists with the same definition is left as it
  is. Running the same manifest every time a file is opened is the intended
  use; the result's `created` lists what was new (`project`, and the ids of
  `entityTypes`, `entities`, `predicates` and `axes`).
- **Never overwrites.** Anything that exists with another definition refuses
  the whole manifest with `CONFLICT`, and `details.divergences` lists each one
  as `{ kind, id, field }`. To change something, use its own operation.
- **Entities.** An existing entity only has to have the same `typeId`. Its
  name, aliases and fields change during play (`entity.update`) and are
  neither compared nor rewritten.

Duplicate ids inside a manifest, a reserved predicate or axis id, and an
entity field its type does not define are `VALIDATION_FAILED`; an entity whose
`typeId` is neither stored nor in the manifest is `NOT_FOUND`. The manifest
uses the engine's own entity ids; `entity.ensureBatch` remains the way to
register entities owned by another system, by `externalRef`.

Creation and update operations return the complete stored object. The generated operation contract is [schema/umwelt-protocol-v1.schema.json](schema/umwelt-protocol-v1.schema.json); its `x-ops` map is authoritative for payload/result definitions and capability kind.

## Process behavior

`umwelt serve <file.umw> [--create] [--durability normal|full]` owns the file lock for the session. A line larger than 16 MiB produces `PAYLOAD_TOO_LARGE`; the server discards the remainder of that line and processes the next one. `shutdown` checkpoints the WAL with `TRUNCATE`, responds successfully, and exits with code 0. EOF performs the same checkpoint and exits without another envelope. A stdout I/O failure is logged to stderr and exits with code 2.

`umwelt call <file.umw> <op> <json>` opens one file, performs an implicit handshake, prints the requested response, and exits. `umwelt create <file.umw>` creates a new file. `umwelt inspect <file.umw>` prints its storage version, contract version, and projects.

## Durability

`--durability` selects how hard SQLite works to keep committed transactions:

- `normal` (default): WAL with `synchronous=NORMAL`. An application crash loses nothing that was committed. A power loss or operating-system crash may lose the most recent commits. The file is never corrupted.
- `full`: WAL with `synchronous=FULL`. Every commit is flushed to disk before its response is written.

The same choice is available as `Durability` on `SqliteStore::create_with`/`open_with` and as `umwelt_open_with_options` in the C ABI. Durability is a property of the open connection, not of the file.
