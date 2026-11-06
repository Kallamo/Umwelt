# Changelog

All notable changes to Umwelt are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [semantic versioning](spec/versioning.md): every package of a release
shares its version.

## [0.1.0] - Unreleased

The first public release.

### Engine

- A ledger of Facts (canon), Holdings (what a holder knows, believes,
  suspects or remembers), Awareness and Dispositions, written in events and
  changed only by transitions that keep the history.
- Bitemporal reads: every query answers at a position and a recorded version,
  for a perspective. Additional time axes, such as reading order next to story
  order, with reads on any axis.
- Knowledge operations: transmitting what one holder told another, witnesses
  of a scene, group coverage, second-order beliefs, secrets and breaches,
  continuity checks of an utterance, and context admission with a reason for
  every refusal.
- Contradictions between beliefs and canon are kept and reported, never
  corrected.
- Draft timelines that can be nested, promoted, rebased, discarded and purged.
- Proposals reviewed per operation before they are applied.
- Full-text search, and semantic search over vectors from the client's own
  embedding model, in text, semantic or hybrid mode, within the reader's
  perspective.
- Atomic, idempotent writes: the same `attemptId` is applied exactly once.
- Storage in a portable SQLite `.umw` file that holds one or more isolated
  projects.

### Interfaces

- JSON protocol v1 over stdio (`umwelt serve`) and over a C ABI, with JSON
  Schemas for every payload and result.
- The `umwelt` command-line tool.
- `@kallamo/umwelt`, the TypeScript client, with helpers that fill and search
  vectors with the application's embedding model.
- `Kallamo.Umwelt`, the .NET binding, running the engine in-process on .NET 6
  and later.
- Language-neutral conformance cases run by every client.

### License

- AGPL-3.0-only, with the Umwelt Free Play Exception: free, non-monetized
  games, mods, interactive fiction and tabletop play tools may use Umwelt
  without releasing their own code, provided they credit it.
