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

Commands are `project.create`, `entityType.create`, `entity.create`, `entity.update`, `entity.archive`, `predicate.define`, `axis.define`, `embedding.defineSpace`, `embedding.dropSpace`, `embedding.put`, `embedding.delete`, `evidence.register`, `commit`, `commitBatch`, `propose`, `proposal.review`, `proposal.apply`, `proposal.reject`, `transmit`, `transmitBatch`, `witness`, `holding.adjustBatch`, `transition.batch`, `disposition.setBatch`, `entity.ensureBatch`, `proposal.rebase`, `timeline.openDraft`, `timeline.promote`, `timeline.rebase`, `timeline.discard`, `timeline.purge`, and `shutdown`.

Queries are `project.get`, `project.list`, `entityType.list`, `entity.get`, `entity.list`, `predicate.list`, `axis.list`, `embedding.listSpaces`, `embedding.pending`, `evidence.get`, `proposal.get`, `proposal.list`, `holds`, `holdings`, `whoHolds`, `canon`, `perspective`, `compare`, `contradictions`, `trace`, `timeline`, `record`, `changes`, `search`, `timeline.get`, `timeline.list`, `entity.resolve`, `entity.resolveBatch`, `entity.at`, `entity.asKnownBy`, `groupCoverage`, `beliefsAbout`, `breaches`, `check.utterance`, and `context.admit`.

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

Creation and update operations return the complete stored object. The generated operation contract is [schema/umwelt-protocol-v1.schema.json](schema/umwelt-protocol-v1.schema.json); its `x-ops` map is authoritative for payload/result definitions and capability kind.

## Process behavior

`umwelt serve <file.umw> [--create] [--durability normal|full]` owns the file lock for the session. A line larger than 16 MiB produces `PAYLOAD_TOO_LARGE`; the server discards the remainder of that line and processes the next one. `shutdown` checkpoints the WAL with `TRUNCATE`, responds successfully, and exits with code 0. EOF performs the same checkpoint and exits without another envelope. A stdout I/O failure is logged to stderr and exits with code 2.

`umwelt call <file.umw> <op> <json>` opens one file, performs an implicit handshake, prints the requested response, and exits. `umwelt create <file.umw>` creates a new file. `umwelt inspect <file.umw>` prints its storage version, contract version, and projects.

## Durability

`--durability` selects how hard SQLite works to keep committed transactions:

- `normal` (default): WAL with `synchronous=NORMAL`. An application crash loses nothing that was committed. A power loss or operating-system crash may lose the most recent commits. The file is never corrupted.
- `full`: WAL with `synchronous=FULL`. Every commit is flushed to disk before its response is written.

The same choice is available as `Durability` on `SqliteStore::create_with`/`open_with` and as `umwelt_open_with_options` in the C ABI. Durability is a property of the open connection, not of the file.
