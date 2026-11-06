<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Query semantics

All temporal reads use both a timeline tick and a recorded version. Missing `At.tick` and `At.asOfVersion` values resolve to the current maximum tick and head version of the timeline being read.

## Timelines

Every read takes an optional `timelineId` (default `main`). Reading main sees main rows only. Reading a draft sees the draft's rows plus each ancestor's rows recorded up to the `baseVersion` of its child on the chain down to the draft (for a draft of main: main rows up to the draft's `baseVersion`); `At.asOfVersion` cannot exceed the draft's head version. `trace` and `record` apply the same visibility, without temporal coordinates. Reading a promoted draft returns `INVALID_STATE` with `details.promotedInto` (the timeline it was promoted into); reading a purged draft returns `NOT_FOUND`. See [timelines.md](timelines.md).

Reads of main at its head use the materialized current status; every other read, and every read of a draft, checks transitions directly.

## Time axes

A project has one main time axis, `Position.tick`, and may declare additional axes with `axis.define` (`{ projectId, axis: { id, label } }`; `axis.list` returns them). The id `tick` and ids beginning with `umwelt:` are reserved. Defining an existing axis with the same label returns it unchanged; another label fails with `CONFLICT`. Axes are never renamed or removed, and they belong to the project: a definition applies at once to every timeline, drafts included.

An event may carry a position on any defined axis in `on` (`"on": { "reading": 12 }`, each value `>= 0`); naming an undefined axis fails with `VALIDATION_FAILED`. Operations that build their own event (`transmit`, `holding.adjustBatch`, `transition.batch`, `disposition.setBatch`) take `on` next to `position`. Records and transitions inherit the positions of the event they were written in; they have none of their own. A position is never corrected: a wrong one is answered by writing a new event.

`At.axis` names the axis a read is on. Without it the read is on the main axis, as before. A read at `(axis, tick, asOfVersion)` sees a record or transition when its event has a position on `axis` up to `tick`, it was recorded up to `asOfVersion`, and it is visible in the timeline read. **A record whose event has no position on the axis is visible to nobody in that read.** The main-axis tick plays no part in a read on another axis, and the other way around. Without `At.tick`, a read on an axis uses the highest position on it among the events the timeline sees (a draft sees its own events and its ancestors' up to each base version), or 0 when there is none. An undefined axis fails with `VALIDATION_FAILED`.

Every query that takes `at` accepts the axis. "Later" in the `future` verdicts of `check.utterance` and `context.admit` means later on the read's axis; `context.admit` refuses a candidate whose event has no position on that axis with `not-on-axis`. `compare` takes an optional `atB`, the position of holder `b` (default: `at`), so that holders can be compared on different axes: what the reader knows in chapter 12 against what a character knows at story tick 10.

Pages keep the main-axis order `(tick, recordedVersion, recordId)` on every axis; on an additional axis that order carries no meaning, and the axis decides only what enters the page. Likewise the "latest" record a query reports (the Holding behind `holds`, the name `entity.at` shows) is the latest in main-axis order. The exception is the event page, `timeline`: on an additional axis it is ordered by the position on that axis, the reading order. Reads on an additional axis always check transitions directly, like draft reads.

## Pagination cursors

Record and event list cursors are opaque and stable. They preserve the ordering `(tick, recordedVersion, recordId)` (for an event page on an additional axis, the tick is the position on that axis) and therefore remain valid when later writes are added to the ledger.

Entity cursors preserve entity identifier ordering. Change-feed continuation uses its monotonic sequence number, which is global across timelines; `timelineId` only filters the items. A purge leaves gaps in the sequence.

Search is the exception. Its opaque cursor represents an FTS result offset. If writes occur between search pages, SQLite may reorder the ranked result set, so the next page can skip or repeat items. Clients that require a stable search snapshot should keep the database unchanged while consuming the pages or restart the search after a write.

## Search

Search by words, by meaning (vectors from the client's embedding model) or both is described in [search.md](search.md).

Without a holder filter, search exposes only active Facts and visible Events. With a holder filter, it exposes only active, available Holdings belonging to that holder. Suppressed and inaccessible Holdings are never returned by search.

## Group membership

`whoHolds` accepts `filter.memberOf`. Membership is read from active
`umwelt:member-of` Facts at the read position, so a member who left before the
position is not counted. `groupCoverage` reports members with and without a
claim; see [knowledge-operations.md](knowledge-operations.md).
