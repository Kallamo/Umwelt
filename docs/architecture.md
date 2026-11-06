# Runtime architecture

## Layers

```text
Clients (CLI, `@kallamo/umwelt`, `Kallamo.Umwelt` over the C ABI, any stdio client)
        │  JSON protocol v1 envelopes
        ▼
umwelt-protocol ── Dispatcher: handshake, parsing, error codes
        │
umwelt-api ─────── Engine: commands and queries, clock, id generator
        │
umwelt-core ────── contracts, validation, materialization (pure)
        │  Store trait
umwelt-sqlite ──── SQLite storage: atomic writes, indexed reads, search
        │
     file.umw
```

- `umwelt-core` owns the contract (`domain`), commit validation and
  materialization, claim keys and fingerprints. It performs no I/O, reads no
  clock, generates no ids and spawns no threads. Storage reaches it through the
  `Store` and `ValidationView` traits.
- `umwelt-sqlite` is the only code that knows SQL. It owns the file format,
  file locking, durability settings, the timeline visibility rule, reads on
  additional time axes, full-text and semantic search, and every query plan.
- `umwelt-api` owns the public `Engine` facade. It supplies clocks and id
  generators, turns derived requests (`transmit`, `witness`, batch maintenance)
  into commits, and computes the queries that combine several reads
  (`groupCoverage`, `check.utterance`, `context.admit`...).
- `umwelt-protocol` owns the newline-delimited JSON protocol: envelopes,
  handshake, capabilities, the operation table (`x-ops`) and error mapping.
- `apps/cli` serves the protocol over stdio (`umwelt serve`), runs one-shot
  calls and generates benchmark fixtures.
- `umwelt-ffi` carries the same protocol through a C ABI, so a host can run the
  engine in-process (a game, a .NET mod).
- `packages/umwelt` is the TypeScript client, generated from the JSON
  schemas. It serializes protocol requests and never implements narrative rules.
- `bindings/csharp` is the official .NET binding: types and a typed method per
  operation generated from the same schemas, over the C ABI.

No client-specific logic lives in any of these layers. Applications, games and
mods are separate projects that speak the protocol.

## Storage safety

Every write is one immediate SQLite transaction: the event, its records,
transitions, claim and side indexes, full-text entries, attempt and change-feed
item are written together or not at all. A replayed `attemptId` with the same
fingerprint returns the stored outcome; a different payload is a `CONFLICT`.

Files carry an application id and a storage version. A file of a newer
storage version than the engine knows is refused; see `spec/versioning.md`.
Every record is one row rebuilt from columns (see `spec/storage.md`).
A lock file gives one process exclusive access. WAL is used with
`synchronous=NORMAL` by default or `FULL` on request (see `spec/protocol.md`).

## Epistemic invariants

The invariants in [`spec/invariants.md`](../spec/invariants.md) are enforced
by tests in `crates/umwelt-api/tests` and by the language-neutral cases in
`conformance/cases`:

- Nobody knows anything without a record whose holder they are. Being present
  at an event grants nothing; witnessing is recorded per holder.
- Canon (Facts) never enters a subjective projection.
- A belief that contradicts canon stays held; contradictions are reported.
- Group knowledge is the group's own records, never the union of its members'.
- Reads at `(axis, tick, asOfVersion)` never see anything later, and reads of one
  timeline never see another's rows except its ancestors' up to each branch
  point.
- Confirmed history is never deleted; state changes are transitions.
- The engine applies no policy and runs no model: decay, trust, secrecy rules
  and embedding vectors come from the client; the engine records and reports.

## Timelines

Main is the confirmed history. Drafts branch from main or from another draft
(at most four deep), see each ancestor up to the version their chain was opened
from, and are promoted into their parent (atomically, only if the parent did
not move), rebased (replayed on the parent's new head) or discarded and purged.
See `spec/timelines.md`.

## Time axes and search

Positions are on the main axis (`tick`) unless a read names an additional axis
declared with `axis.define`; a read on an axis sees only records whose event
has a position on it. Search is full-text, semantic over vectors the client's
model made, or both, and applies the same visibility as every other read. See
`spec/queries.md` and `spec/search.md`.

## Context admission

`context.admit` checks candidates in a fixed order that is part of the
contract: candidate shape, project and timeline, time, status, perspective,
accessibility, classification, duplication, then budget. A candidate reports
the first check it fails, as a machine-readable reason. Admitting withheld
content requires an authorized narrator; asking for it without one is a
validation failure. See `spec/knowledge-operations.md`.
