<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Specification

This directory defines how Umwelt behaves: what each operation accepts and
returns, what a read can see, and what the engine guarantees. Client libraries
and applications rely on it; a change in behavior updates it.

The generated JSON Schemas in [`schema/`](schema) are authoritative for the
shape of every payload and result. These pages explain the rules the shapes
cannot express. For an introduction, start with
[../docs/getting-started.md](../docs/getting-started.md) and
[../docs/concepts.md](../docs/concepts.md).

## Reading order

1. [invariants.md](invariants.md): the guarantees every operation keeps.
2. [protocol.md](protocol.md): envelopes, the handshake and the list of
   operations.
3. [queries.md](queries.md): how reads resolve a position and a version,
   timelines, time axes and pagination.
4. [timelines.md](timelines.md): draft timelines, their visibility, promote,
   rebase, discard and purge.
5. [knowledge-operations.md](knowledge-operations.md): witnesses, groups,
   batch maintenance, names over time, second-order knowledge, secrets,
   continuity checks and context admission.
6. [search.md](search.md): full-text, semantic and hybrid search, and the
   vectors that come from the client's embedding model.
7. [errors.md](errors.md): the protocol error codes.

## Reference

- [claim-key.md](claim-key.md): how claim and slot keys are computed, with a
  fixed test vector.
- [storage.md](storage.md): what a `.umw` file holds and why.
- [versioning.md](versioning.md): product, contract, protocol, storage and C
  ABI versions, and what counts as an incompatible change.
- [`schema/umwelt-v1.schema.json`](schema/umwelt-v1.schema.json): the data
  contract.
- [`schema/umwelt-protocol-v1.schema.json`](schema/umwelt-protocol-v1.schema.json):
  every operation's payload and result (`x-ops`).
