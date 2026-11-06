<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Storage format

A `.umw` file is a SQLite database with `application_id` `0x554D5754` and
`user_version` equal to the storage version, currently **5**, which the
`hello` result reports. A file of a newer storage version is refused with
`UNSUPPORTED_FILE_VERSION`; see [versioning.md](versioning.md#storage).

The layout is an implementation detail of `umwelt-sqlite`: clients use the
protocol, never SQL. This page explains what the file holds and why it is
shaped this way.

## Principle: nothing stored twice

Each fact is stored once. A record row holds its common fields, its claim, its
acquisition and its holder in typed columns, and `extra` holds only what no
query filters on. The engine rebuilds the public JSON from the columns when it
reads, so no record, event or transition is kept a second time as JSON.

| Table | One row per | Notes |
| --- | --- | --- |
| `records` | Fact, Holding, Awareness or Disposition | Claims and acquisitions are columns of the record. `WITHOUT ROWID`. |
| `events` | event | Type, title and provenance as columns; description, participants, location and evidence in `extra`. |
| `transitions` | transition | Fully columnar. |
| `provenances` | distinct provenance | Shared by every row with the same provenance. |
| `attempts` | applied attempt | 32-byte fingerprint, event, version; the outcome's ids, warnings and preconditions in `extra`. |
| `change_feed` | change item | Commit items as columns; the payload is rebuilt when read. |
| `current_status` | transitioned record | The latest status, for fast reads of main at its head. |
| `event_positions` | event position on an additional axis | Key `(project_id, event_id, axis_id)`. The only place a position is stored; its `recorded_version` is the event's, kept so event pages by axis read one ordered index. |
| `axes` | additional axis | Id and label. |
| `embedding_spaces` | embedding space | Id, dimensions and model label. |
| `embeddings` | vector of a searchable document in a space | Unique index `(project_id, space_id, doc_type, doc_id)`. Little-endian `f32`, normalized; the first 16 bytes of the SHA-256 of the text it was made from. A rowid table: a vector is too large for a `WITHOUT ROWID` row. Derived data, no timeline column. |
| `timelines` | draft | Main lives on the project row. `parent_id` names the parent (main or a draft), indexed to find a draft's children. |
| `search_fts` | searchable text | FTS5. |

Entities, entity types, predicates, evidence sources and proposals are stored as
JSON documents: they are few and are read whole.

## Provenance

Most workloads repeat the same provenance thousands of times (every transmit
has the same one). A provenance is stored once per project; its id is the first
8 bytes of the SHA-256 of its JSON, so the same provenance always has the same
id, and an id read once never changes meaning.

## Keys

Claim and slot keys are stored as the first 16 bytes of their SHA-256 digest
(see [claim-key.md](claim-key.md#storage-note)).

## Indexes

Every paginated claim index ends in `(tick, recorded_version, id)`, so a page is
read in index order. Partial indexes keep rare rows cheap to find: Facts,
withheld Holdings, secrets, told-by sources, witnessed events, and the rows of
drafts. See [timelines.md](timelines.md) for how timelines are read.

A read on an additional axis keeps the same indexes and order: it replaces the
tick bound of each row and of its transitions with a primary-key lookup in
`event_positions`. Event pages on an axis read
`event_positions_order (project_id, axis_id, tick, recorded_version, event_id)`.

## Versions

The storage version changes whenever the layout does. How files of an earlier
version are migrated, and why a newer one is refused, is described in
[versioning.md](versioning.md#storage).
