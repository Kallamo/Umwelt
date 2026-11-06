-- SPDX-License-Identifier: AGPL-3.0-only
-- Storage version 5. Records, claims and acquisitions share one table and
-- every record is rebuilt from columns; repeated provenance is stored once;
-- drafts branch from main or from other drafts; events may have positions on
-- additional time axes; searchable documents may have vectors from the
-- client's embedding models. See spec/storage.md.
PRAGMA application_id = 0x554D5754;
PRAGMA user_version = 5;

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    timescale TEXT NOT NULL,
    vocabulary_mode TEXT NOT NULL,
    created_at TEXT NOT NULL,
    -- Head version and highest tick of the main timeline. Every read of main
    -- checks this window, so it stays on the project row.
    version INTEGER NOT NULL,
    max_tick INTEGER NOT NULL DEFAULT 0,
    -- Last version handed out to any timeline. Versions are unique per project
    -- across main and drafts.
    last_version INTEGER NOT NULL DEFAULT 0
);
-- Drafts. Main is implicit: its window lives on the project row. A draft
-- branches from its parent (main or another draft) at base_version.
CREATE TABLE timelines (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    parent_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    base_version INTEGER,
    head_version INTEGER NOT NULL,
    max_tick INTEGER NOT NULL,
    label TEXT,
    created_at TEXT NOT NULL,
    closed_at TEXT,
    purged_at TEXT,
    PRIMARY KEY (project_id, id)
);
-- The drafts opened from a timeline (promote, discard, purge and rebase check
-- them).
CREATE INDEX timelines_parent ON timelines(project_id, parent_id);
CREATE TABLE entity_types (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
);
CREATE TABLE entities (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    type_id TEXT NOT NULL,
    name TEXT NOT NULL,
    external_system TEXT,
    external_id TEXT,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, id),
    UNIQUE (project_id, external_system, external_id)
);
CREATE TABLE predicates (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
);
-- Additional time axes (the main axis, `tick`, is implicit).
CREATE TABLE axes (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
) WITHOUT ROWID;
-- Embedding spaces: the vectors of one client model.
CREATE TABLE embedding_spaces (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    model TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
) WITHOUT ROWID;
-- One vector per searchable document and space: derived data, not history.
-- `vector` is little-endian f32, normalized; `text_hash` is the first 16
-- bytes of the SHA-256 of the text it was made from. No timeline column:
-- visibility comes from the document. A rowid table, not WITHOUT ROWID: a
-- vector is too large for a WITHOUT ROWID row (384 dimensions are 1,536
-- bytes), which would spill every row into its own overflow page.
CREATE TABLE embeddings (
    project_id TEXT NOT NULL,
    space_id TEXT NOT NULL,
    doc_type TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    text_hash BLOB NOT NULL,
    vector BLOB NOT NULL
);
CREATE UNIQUE INDEX embeddings_doc ON embeddings(project_id, space_id, doc_type, doc_id);
CREATE TABLE evidence_sources (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
);
-- Provenance repeats across most records of a workload (every transmit has
-- the same one). It is stored once; its id is derived from its content, so an
-- id never changes meaning.
CREATE TABLE provenances (
    project_id TEXT NOT NULL,
    id INTEGER NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
) WITHOUT ROWID;
CREATE TABLE events (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    timeline_id TEXT NOT NULL,
    tick INTEGER NOT NULL,
    recorded_version INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    title TEXT NOT NULL,
    provenance_id INTEGER NOT NULL,
    -- JSON with the optional fields (description, participants, location,
    -- evidence); NULL when all are empty.
    extra TEXT,
    PRIMARY KEY (project_id, id)
) WITHOUT ROWID;
-- Positions of events on additional axes; the only place they are stored.
-- `recorded_version` is the event's, kept so `event_positions_order` can serve
-- event pages by axis. No timeline column: visibility comes from the event.
CREATE TABLE event_positions (
    project_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    axis_id TEXT NOT NULL,
    tick INTEGER NOT NULL,
    recorded_version INTEGER NOT NULL,
    PRIMARY KEY (project_id, event_id, axis_id)
) WITHOUT ROWID;
-- Every record kind in one row. Columns hold everything queries filter on;
-- `extra` holds the rest (object value, qualifiers, about, told-by intent,
-- inference sources, text, disposition value) and is NULL when empty.
CREATE TABLE records (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    timeline_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    tick INTEGER NOT NULL,
    recorded_version INTEGER NOT NULL,
    event_id TEXT NOT NULL,
    provenance_id INTEGER NOT NULL,
    -- Holdings, Awareness and Dispositions.
    holder_id TEXT,
    -- Facts and Holdings: the claim. Keys are the first 16 bytes of the
    -- SHA-256 claim and slot keys.
    claim_key BLOB,
    slot_key BLOB,
    subject_id TEXT,
    predicate TEXT,
    object_entity_id TEXT,
    polarity INTEGER,
    -- Holdings.
    attitude TEXT,
    confidence REAL,
    accessibility TEXT,
    -- JSON array of who a secret Holding may be shared with; NULL otherwise.
    shared_with TEXT,
    acquisition_kind TEXT,
    source_holder_id TEXT,
    source_record_id TEXT,
    -- The witnessed event or the evidence source.
    acquisition_ref TEXT,
    -- Awareness and Dispositions.
    target_id TEXT,
    dimension TEXT,
    extra TEXT,
    PRIMARY KEY (project_id, id)
) WITHOUT ROWID;
CREATE TABLE transitions (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    timeline_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    status TEXT NOT NULL,
    superseded_by TEXT,
    tick INTEGER NOT NULL,
    recorded_version INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    event_id TEXT NOT NULL,
    provenance_id INTEGER NOT NULL,
    PRIMARY KEY (project_id, id)
) WITHOUT ROWID;
CREATE TABLE current_status (
    project_id TEXT NOT NULL,
    timeline_id TEXT NOT NULL,
    record_id TEXT NOT NULL,
    status TEXT NOT NULL,
    changed_by TEXT NOT NULL,
    changed_tick INTEGER NOT NULL,
    changed_version INTEGER NOT NULL,
    PRIMARY KEY (project_id, timeline_id, record_id)
) WITHOUT ROWID;
CREATE TABLE attempts (
    project_id TEXT NOT NULL,
    timeline_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    -- SHA-256 of the request, 32 bytes.
    fingerprint BLOB NOT NULL,
    event_id TEXT NOT NULL,
    recorded_version INTEGER NOT NULL,
    -- JSON with the outcome's record and transition ids, warnings and the
    -- commit's preconditions; NULL when all are empty.
    extra TEXT,
    PRIMARY KEY (project_id, timeline_id, attempt_id)
) WITHOUT ROWID;
CREATE TABLE proposals (
    project_id TEXT NOT NULL,
    id TEXT NOT NULL,
    status TEXT NOT NULL,
    rationale TEXT,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, id)
);
CREATE TABLE proposal_reviews (
    project_id TEXT NOT NULL,
    proposal_id TEXT NOT NULL,
    operation_index INTEGER NOT NULL,
    seq INTEGER NOT NULL,
    payload TEXT NOT NULL,
    PRIMARY KEY (project_id, proposal_id, operation_index, seq)
);
CREATE TABLE change_feed (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    timeline_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    type TEXT NOT NULL,
    -- A commit item is rebuilt from these columns.
    event_id TEXT,
    record_count INTEGER,
    transition_count INTEGER,
    -- Payload of every other item type.
    payload TEXT
);
CREATE VIRTUAL TABLE search_fts USING fts5(
    project_id UNINDEXED,
    doc_type UNINDEXED,
    doc_id UNINDEXED,
    text,
    tokenize='unicode61 remove_diacritics 2'
);

-- Every paginated claim query orders by (tick, recorded_version, id). Access
-- indexes end with those columns so a page reads its rows in order from the
-- index instead of sorting a holder's whole history.
CREATE INDEX claims_holder_kind_order ON records(project_id, holder_id, kind, tick, recorded_version, id);
CREATE INDEX claims_holder_subject_order ON records(project_id, holder_id, subject_id, tick, recorded_version, id);
CREATE INDEX claims_holder_object_order ON records(project_id, holder_id, object_entity_id, tick, recorded_version, id);
CREATE INDEX claims_holder_key_order ON records(project_id, holder_id, claim_key, tick, recorded_version, id);
CREATE INDEX claims_key_order ON records(project_id, claim_key, tick, recorded_version, id);
CREATE INDEX claims_slot_kind ON records(project_id, slot_key, kind, tick, recorded_version, id);
CREATE INDEX claims_holder_predicate_object ON records(project_id, holder_id, predicate, object_entity_id, tick, recorded_version, id);
CREATE INDEX claims_fact_subject_predicate ON records(project_id, subject_id, predicate, tick, recorded_version, id) WHERE kind = 'fact';
CREATE INDEX claims_fact_order ON records(project_id, tick, recorded_version, id) WHERE kind = 'fact';
CREATE INDEX claims_fact_object ON records(project_id, object_entity_id, predicate, tick, recorded_version, id) WHERE kind = 'fact';
-- Withheld holdings are rare; counting them must not touch the visible ones.
CREATE INDEX claims_withheld ON records(project_id, holder_id, tick) WHERE accessibility IN ('suppressed', 'inaccessible');
-- Secrets are rare; breach reports read only them.
CREATE INDEX claims_secret ON records(project_id, holder_id, tick, recorded_version, id) WHERE shared_with IS NOT NULL;
CREATE INDEX acquisitions_source_record ON records(project_id, source_record_id) WHERE source_record_id IS NOT NULL;
CREATE INDEX acquisitions_witnessed ON records(project_id, acquisition_ref) WHERE acquisition_kind = 'witnessed';
CREATE INDEX transitions_target_window ON transitions(project_id, target_id, tick, recorded_version);
CREATE INDEX events_timeline_order ON events(project_id, timeline_id, tick, recorded_version, id);
CREATE INDEX event_positions_order ON event_positions(project_id, axis_id, tick, recorded_version, event_id);
-- Draft rows only (main rows are never in these): promote and purge find a
-- draft's rows without scanning the whole ledger.
CREATE INDEX records_draft ON records(project_id, timeline_id) WHERE timeline_id <> 'main';
CREATE INDEX transitions_draft ON transitions(project_id, timeline_id) WHERE timeline_id <> 'main';
CREATE INDEX change_feed_draft ON change_feed(project_id, timeline_id) WHERE timeline_id <> 'main';
