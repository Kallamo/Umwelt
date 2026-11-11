<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Invariants

These guarantees hold for every operation, every client and every file. They
are enforced by the engine, covered by the Rust test suite and by the
language-neutral cases in [`conformance/cases`](../conformance/cases), which
every client library runs.

1. **Nobody knows anything without an explicit record whose holder they
   are.** Being present at an event, being aware of someone or belonging to a
   group grants no knowledge.
2. **Canon never appears in a subjective projection.** A Fact is not
   knowledge of that fact; a holder's perspective contains only the holder's
   own records.
3. **Confirmed history is never deleted.** Every state change is a transition
   with a position and a version. Only drafts that were never promoted can be
   purged.
4. **A read at `(axis, tick, asOfVersion)` never sees anything later on that
   axis or in version.** A record whose event has no position on the axis
   read is visible to nobody in that read.
5. **The engine never corrects beliefs.** A belief that contradicts canon
   stays held; the contradiction is reported.
6. **The engine never infers and never applies policy.** Probability, decay,
   distortion, trust and secrecy rules come from the client; the engine
   records and reports. It never runs or calls a model.
7. **Every write is atomic, and the same `attemptId` is applied exactly
   once.** A replay with the same request returns the stored outcome; a
   different request with a used `attemptId` is a `CONFLICT`.
8. **The core does no I/O.** `umwelt-core` reads no clock, generates no ids
   and spawns no threads; storage, clocks and ids are supplied to it.
9. **A read in one timeline never sees another timeline's rows**, except its
   ancestors' rows, each up to the `baseVersion` of its child on the chain down
   to the draft.
10. **The intent of a telling is the narrator's knowledge.** A read made on
    behalf of a holder never reveals the intent with which they were told
    something: it reads `unknown`. The recorded intent stays available to
    narrator reads (`record`, `trace`, `breaches`).

Group knowledge follows from 1: a group knows what the group's own records
say, never the union of its members' knowledge. Vectors used by semantic
search follow from 1 and 4: they only order documents a read would see anyway.
