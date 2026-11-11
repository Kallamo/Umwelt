// SPDX-License-Identifier: AGPL-3.0-only
//! SQLite persistence for the Umwelt v1 ledger.

#[macro_use]
mod rows;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::Duration;

use fs4::fs_std::FileExt;
use rusqlite::functions::FunctionFlags;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, ToSql, Transaction, TransactionBehavior, params,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use umwelt_core::domain::*;
use umwelt_core::{
    ChangePage, DocRef, EVENT_DOC, EntityPage, EntityWrite, EventCursor, EventPage, EventQuery,
    HoldingSummary, MAX_SEMANTIC_WINDOW, PendingPage, PendingQuery, PerspectiveRecords,
    PreparedCommit, RECORD_DOC, RankedDoc, RecordCursor, RecordPage, RecordQuery, Result,
    ScopeAncestor, SearchPage, SearchQuery, SemanticQuery, Store, StoreCounts, TimelineScope,
    UmweltError, UmweltErrorCode, ValidationView, event_search_text, fuse_rankings,
    materialize_commit, rank_order, record_search_text, similarity, text_hash, validate_commit,
};

use rows::{
    AttemptExtra, EVENT_COLUMNS, EventRow, ProvenanceCache, ProvenanceWrites, RECORD_COLUMN_COUNT,
    RECORD_COLUMNS, RecordColumns, RecordRow, TRANSITION_COLUMNS, TransitionRow, fingerprint_blob,
    fingerprint_hex, insert_event_row, insert_transition_row, key_blob, record_common,
};

const APPLICATION_ID: i64 = 0x554D_5754;
/// The layout of a `.umw` file (see `spec/storage.md`). Files of another
/// version are refused.
pub const STORAGE_VERSION: u32 = 5;
const FILE_VERSION: i64 = STORAGE_VERSION as i64;

// Visibility of a row in a timeline (see `TimelineScope`): the timeline's own
// rows, plus each ancestor's rows up to the version bound to it. Ancestor slots
// a draft does not use, and all of them for main, are bound to NULL, which
// reduces the expression to `timeline_id = :timeline_id`. No index contains
// `timeline_id`, so this is always a residual filter and never changes which
// index a query uses. `ScopeBinding` binds it.
macro_rules! visible {
    ($alias:literal) => {
        concat!(
            "(",
            $alias,
            ".timeline_id = :timeline_id OR (",
            $alias,
            ".timeline_id = :a1 AND ",
            $alias,
            ".recorded_version <= :v1) OR (",
            $alias,
            ".timeline_id = :a2 AND ",
            $alias,
            ".recorded_version <= :v2) OR (",
            $alias,
            ".timeline_id = :a3 AND ",
            $alias,
            ".recorded_version <= :v3) OR (",
            $alias,
            ".timeline_id = :a4 AND ",
            $alias,
            ".recorded_version <= :v4))"
        )
    };
}

// The tick bound of a read on an additional axis: the row's event has a
// position on `:axis` up to `:tick`, found through the primary key of
// `event_positions`. A row whose event has no position there is invisible.
// `bound_on_axis` puts it in place of every main-axis bound
// (`r.tick <= :tick`, `t.tick <= :tick`, `e.tick <= :tick`), so positioned
// SQL is written once, for the main axis, with exactly that bound text.
macro_rules! on_axis {
    ($alias:literal, $event_id:literal) => {
        concat!(
            "EXISTS (SELECT 1 FROM event_positions p WHERE p.project_id = ",
            $alias,
            ".project_id AND p.event_id = ",
            $alias,
            ".",
            $event_id,
            " AND p.axis_id = :axis AND p.tick <= :tick)"
        )
    };
}

/// A positioned query as written (main axis), or bounded on an additional
/// axis instead (see `on_axis!`). Indexes and order do not change. Without
/// the tick range the planner would check transitions through the primary key
/// (every transition of the project), so the axis variant names their index.
fn bound_on_axis(sql: &str, axis: bool) -> Cow<'_, str> {
    if !axis {
        return Cow::Borrowed(sql);
    }
    Cow::Owned(
        sql.replace("r.tick <= :tick", on_axis!("r", "event_id"))
            .replace("t.tick <= :tick", on_axis!("t", "event_id"))
            .replace("e.tick <= :tick", on_axis!("e", "id"))
            .replace(
                "FROM transitions t ",
                "FROM transitions t INDEXED BY transitions_target_window ",
            ),
    )
}

const HOLDER_RECORDS_CURRENT: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r INDEXED BY claims_holder_kind_order \
     WHERE r.project_id = :project_id AND r.holder_id = :holder AND r.tick <= :tick \
     AND r.recorded_version <= :version AND r.timeline_id = 'main' \
     AND NOT EXISTS (SELECT 1 FROM current_status s WHERE s.project_id = r.project_id \
     AND s.timeline_id = 'main' AND s.record_id = r.id) \
     ORDER BY r.recorded_version, r.id"
);
const HOLDER_RECORDS_HISTORICAL: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r INDEXED BY claims_holder_kind_order \
     WHERE r.project_id = :project_id AND r.holder_id = :holder AND r.tick <= :tick \
     AND r.recorded_version <= :version AND ",
    visible!("r"),
    " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id AND t.target_id = r.id \
     AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    ") ORDER BY r.recorded_version, r.id"
);
const ALL_RECORDS_CURRENT: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r \
     WHERE r.project_id = :project_id AND r.timeline_id = 'main' AND r.tick <= :tick \
     AND r.recorded_version <= :version \
     AND NOT EXISTS (SELECT 1 FROM current_status s WHERE s.project_id = r.project_id \
     AND s.timeline_id = 'main' AND s.record_id = r.id) \
     ORDER BY r.recorded_version, r.id"
);
const ALL_RECORDS_HISTORICAL: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r \
     WHERE r.project_id = :project_id AND r.tick <= :tick AND r.recorded_version <= :version AND ",
    visible!("r"),
    " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id AND t.target_id = r.id \
     AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    ") ORDER BY r.recorded_version, r.id"
);

/// Event pages. Main reads one ordered range of `events_timeline_order`. A
/// draft reads one ordered range per timeline of its scope (its own events and
/// each ancestor's up to its bound; unused slots are NULL and read nothing) and
/// merges them under a final sort bounded by five times the page size.
const EVENTS_MAIN: &str = concat!(
    "SELECT ",
    event_columns!(),
    " FROM events e \
     WHERE e.project_id = ?1 AND e.timeline_id = ?2 AND e.tick <= ?3 AND e.recorded_version <= ?4 \
     AND (?5 IS NULL OR e.tick > ?5 OR (e.tick = ?5 AND e.recorded_version > ?6) \
     OR (e.tick = ?5 AND e.recorded_version = ?6 AND e.id > ?7)) \
     ORDER BY e.tick, e.recorded_version, e.id LIMIT ?8"
);
/// One ordered range of an event page in a draft (see `EVENTS_DRAFT`).
macro_rules! event_range {
    ($timeline:literal, $bound:literal) => {
        concat!(
            "SELECT * FROM (SELECT ",
            event_columns!(),
            " FROM events e \
             WHERE e.project_id = :project_id AND e.timeline_id = ",
            $timeline,
            " AND e.tick <= :tick AND e.recorded_version <= :version",
            $bound,
            " AND (:cursor_tick IS NULL OR e.tick > :cursor_tick \
             OR (e.tick = :cursor_tick AND e.recorded_version > :cursor_version) \
             OR (e.tick = :cursor_tick AND e.recorded_version = :cursor_version \
             AND e.id > :cursor_id)) \
             ORDER BY e.tick, e.recorded_version, e.id LIMIT :limit)"
        )
    };
}
const EVENTS_DRAFT: &str = concat!(
    "SELECT * FROM (",
    event_range!(":timeline_id", ""),
    " UNION ALL ",
    event_range!(":a1", " AND e.recorded_version <= :v1"),
    " UNION ALL ",
    event_range!(":a2", " AND e.recorded_version <= :v2"),
    " UNION ALL ",
    event_range!(":a3", " AND e.recorded_version <= :v3"),
    " UNION ALL ",
    event_range!(":a4", " AND e.recorded_version <= :v4"),
    ") ORDER BY tick, recorded_version, id LIMIT :limit"
);

/// An event page on an additional axis, in reading order: one ordered range of
/// `event_positions_order`, each event checked against the timeline scope.
const EVENTS_ON_AXIS: &str = concat!(
    "SELECT ",
    event_columns!(),
    " FROM event_positions p INDEXED BY event_positions_order \
     CROSS JOIN events e ON e.project_id = p.project_id AND e.id = p.event_id \
     WHERE p.project_id = :project_id AND p.axis_id = :axis AND p.tick <= :tick \
     AND p.recorded_version <= :version AND ",
    visible!("e"),
    " AND (:cursor_tick IS NULL OR (p.tick, p.recorded_version, p.event_id) > \
     (:cursor_tick, :cursor_version, :cursor_id)) \
     ORDER BY p.tick, p.recorded_version, p.event_id LIMIT :limit"
);
/// The highest position on an axis among the events visible in a timeline.
const AXIS_MAX_TICK: &str = concat!(
    "SELECT p.tick FROM event_positions p INDEXED BY event_positions_order \
     CROSS JOIN events e ON e.project_id = p.project_id AND e.id = p.event_id \
     WHERE p.project_id = :project_id AND p.axis_id = :axis AND ",
    visible!("e"),
    " ORDER BY p.tick DESC LIMIT 1"
);

/// Moves a draft's rows into its parent (`?1` project, `?2` draft, `?3`
/// parent). Every statement reaches the draft's rows through an index: the
/// timeline index on events, the primary key on attempts and current_status,
/// and the partial draft-only indexes elsewhere (hence the redundant
/// `timeline_id <> 'main'`).
const PROMOTE_STATEMENTS: [&str; 7] = [
    "UPDATE events SET timeline_id = ?3 WHERE project_id = ?1 AND timeline_id = ?2",
    "UPDATE records SET timeline_id = ?3 \
     WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main'",
    "UPDATE transitions SET timeline_id = ?3 \
     WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main'",
    "UPDATE attempts SET timeline_id = ?3 WHERE project_id = ?1 AND timeline_id = ?2",
    "UPDATE change_feed SET timeline_id = ?3 \
     WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main'",
    // Same tie rule as a transition written directly to the parent. `AND true`
    // keeps SQLite from parsing ON CONFLICT as part of the SELECT's join.
    "INSERT INTO current_status (project_id, timeline_id, record_id, status, changed_by, changed_tick, changed_version) \
     SELECT project_id, ?3, record_id, status, changed_by, changed_tick, changed_version \
     FROM current_status WHERE project_id = ?1 AND timeline_id = ?2 AND true \
     ON CONFLICT(project_id, timeline_id, record_id) DO UPDATE SET status = excluded.status, \
     changed_by = excluded.changed_by, changed_tick = excluded.changed_tick, \
     changed_version = excluded.changed_version \
     WHERE excluded.changed_tick > current_status.changed_tick \
     OR (excluded.changed_tick = current_status.changed_tick \
     AND excluded.changed_version >= current_status.changed_version)",
    "DELETE FROM current_status WHERE project_id = ?1 AND timeline_id = ?2",
];

/// Deletes a discarded draft's rows (`?1` project, `?2` draft). Rows found
/// through the draft's records and events go first. The full-text deletes scan
/// `search_fts` (FTS5 cannot index its UNINDEXED columns); purges are rare.
/// Deletes the vectors of a purged draft's documents. A rebase keeps them:
/// it writes the same ids again.
const PURGE_EMBEDDINGS: [&str; 2] = [
    "DELETE FROM embeddings WHERE project_id = ?1 \
     AND space_id IN (SELECT id FROM embedding_spaces WHERE project_id = ?1) \
     AND doc_type = 'record' AND doc_id IN \
     (SELECT id FROM records WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main')",
    "DELETE FROM embeddings WHERE project_id = ?1 \
     AND space_id IN (SELECT id FROM embedding_spaces WHERE project_id = ?1) \
     AND doc_type = 'event' AND doc_id IN \
     (SELECT id FROM events WHERE project_id = ?1 AND timeline_id = ?2)",
];
const PURGE_STATEMENTS: [&str; 4] = [
    "DELETE FROM event_positions WHERE project_id = ?1 AND event_id IN \
     (SELECT id FROM events WHERE project_id = ?1 AND timeline_id = ?2)",
    "DELETE FROM search_fts WHERE project_id = ?1 AND doc_type = 'record' AND doc_id IN \
     (SELECT id FROM records WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main')",
    "DELETE FROM search_fts WHERE project_id = ?1 AND doc_type = 'event' AND doc_id IN \
     (SELECT id FROM events WHERE project_id = ?1 AND timeline_id = ?2)",
    "DELETE FROM current_status WHERE project_id = ?1 AND timeline_id = ?2",
];
const PURGE_CHANGE_FEED: &str =
    "DELETE FROM change_feed WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main'";
const PURGE_TRANSITIONS: &str =
    "DELETE FROM transitions WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main'";
const PURGE_RECORDS: &str =
    "DELETE FROM records WHERE project_id = ?1 AND timeline_id = ?2 AND timeline_id <> 'main'";
const PURGE_EVENTS: &str = "DELETE FROM events WHERE project_id = ?1 AND timeline_id = ?2";
const PURGE_ATTEMPTS: &str = "DELETE FROM attempts WHERE project_id = ?1 AND timeline_id = ?2";

/// Secret Holdings at the read position, through the partial `claims_secret`
/// index: the cost follows the number of secrets, not of Holdings. These
/// lookups name their index: otherwise the planner may start from the
/// timeline condition and walk every record up to the tick.
const SECRETS: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r INDEXED BY claims_secret \
     WHERE r.project_id = :project_id AND r.shared_with IS NOT NULL \
     AND r.tick <= :tick AND r.recorded_version <= :version AND ",
    visible!("r"),
    " ORDER BY r.tick, r.recorded_version, r.id"
);
const SECRETS_OF_HOLDER: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r INDEXED BY claims_secret \
     WHERE r.project_id = :project_id AND r.shared_with IS NOT NULL AND r.holder_id = :holder \
     AND r.tick <= :tick AND r.recorded_version <= :version AND ",
    visible!("r"),
    " ORDER BY r.tick, r.recorded_version, r.id"
);
/// Holdings acquired by being told one source record.
const TOLD_FROM: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r INDEXED BY acquisitions_source_record \
     WHERE r.project_id = :project_id AND r.source_record_id = :source \
     AND r.tick <= :tick AND r.recorded_version <= :version AND ",
    visible!("r"),
    " ORDER BY r.tick, r.recorded_version, r.id"
);
/// The current Disposition of a holder per target and dimension: the newest
/// active one, optionally for one target and/or one dimension.
const DISPOSITIONS: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM (SELECT r.id AS ranked_id, ROW_NUMBER() OVER \
     (PARTITION BY r.target_id, r.dimension \
     ORDER BY r.tick DESC, r.recorded_version DESC, r.id DESC) AS position \
     FROM records r INDEXED BY claims_holder_kind_order \
     WHERE r.project_id = :project_id AND r.holder_id = :holder AND r.kind = 'disposition' \
     AND (:target IS NULL OR r.target_id = :target) \
     AND (:dimension IS NULL OR r.dimension = :dimension) \
     AND r.tick <= :tick AND r.recorded_version <= :version AND ",
    visible!("r"),
    " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id \
     AND t.target_id = r.id AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    ")) ranked CROSS JOIN records r ON r.project_id = :project_id AND r.id = ranked.ranked_id \
     WHERE ranked.position = 1 ORDER BY r.target_id, r.dimension"
);
/// Whether a holder has an active, available Holding witnessed at an event.
const WITNESSED_BY: &str = concat!(
    "SELECT EXISTS (SELECT 1 FROM records r INDEXED BY acquisitions_witnessed \
     WHERE r.project_id = :project_id AND r.acquisition_kind = 'witnessed' \
     AND r.acquisition_ref = :event_id \
     AND r.holder_id = :holder AND r.tick <= :tick AND r.recorded_version <= :version \
     AND (r.accessibility IS NULL OR r.accessibility = 'available') AND ",
    visible!("r"),
    " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id \
     AND t.target_id = r.id AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    "))"
);

/// A visible record at a position.
const RECORD_AT: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r WHERE r.project_id = :project_id AND r.id = :id \
     AND r.tick <= :tick AND r.recorded_version <= :version AND ",
    visible!("r")
);
/// A visible record whatever its position.
const RECORD_BY_ID: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r WHERE r.project_id = :project_id AND r.id = :id AND ",
    visible!("r")
);
const EVENT_BY_ID: &str = concat!(
    "SELECT ",
    event_columns!(),
    " FROM events e WHERE e.project_id = :project_id AND e.id = :id AND ",
    visible!("e")
);
/// The latest visible transition of a record at a position.
const LATEST_TRANSITION: &str = concat!(
    "SELECT ",
    transition_columns!(),
    " FROM transitions t WHERE t.project_id = :project_id \
     AND t.target_id = :id AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    " ORDER BY t.tick DESC, t.recorded_version DESC, t.ordinal DESC LIMIT 1"
);

/// How hard SQLite works to keep committed transactions across failures.
///
/// Both modes keep the file consistent: a crash never corrupts it. They differ
/// only in what a power loss or operating-system crash can take back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Durability {
    /// `synchronous=NORMAL` in WAL mode. Survives an application crash; after a
    /// power loss the most recent committed transactions may be lost. Default.
    #[default]
    Normal,
    /// `synchronous=FULL`. Every commit is flushed to disk before it returns.
    Full,
}

impl Durability {
    fn pragma_value(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Full => "FULL",
        }
    }
}

#[derive(Debug)]
pub struct SqliteStore {
    connection: Connection,
    /// Provenance values read so far; see `rows::ProvenanceCache`.
    provenances: ProvenanceCache,
    _lock: Option<File>,
}

impl SqliteStore {
    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory().map_err(database_error)?;
        configure_connection(&connection, None)?;
        connection
            .execute_batch(include_str!("schema.sql"))
            .map_err(database_error)?;
        Ok(Self {
            connection,
            provenances: ProvenanceCache::default(),
            _lock: None,
        })
    }

    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        Self::create_with(path, Durability::default())
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(path, Durability::default())
    }

    pub fn create_with(path: impl AsRef<Path>, durability: Durability) -> Result<Self> {
        let path = validate_path(path.as_ref())?;
        if path.exists() {
            return Err(UmweltError::public(
                UmweltErrorCode::Conflict,
                format!("File '{}' already exists.", path.display()),
            ));
        }
        let lock = acquire_lock(&path)?;
        let connection = Connection::open(&path).map_err(database_error)?;
        configure_connection(&connection, Some(durability))?;
        connection
            .execute_batch(include_str!("schema.sql"))
            .map_err(database_error)?;
        Ok(Self {
            connection,
            provenances: ProvenanceCache::default(),
            _lock: Some(lock),
        })
    }

    pub fn open_with(path: impl AsRef<Path>, durability: Durability) -> Result<Self> {
        let path = validate_path(path.as_ref())?;
        if !path.exists() {
            return Err(UmweltError::not_found(format!(
                "Umwelt file '{}' does not exist.",
                path.display()
            )));
        }
        let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(database_error)?;
        let lock = acquire_lock(&path)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_error)?;
        let application_id: i64 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(database_error)?;
        if application_id != APPLICATION_ID {
            return Err(UmweltError::public(
                UmweltErrorCode::NotUmweltFile,
                "The file is not an Umwelt database.",
            ));
        }
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(database_error)?;
        if version != FILE_VERSION {
            let relation = if version > FILE_VERSION {
                "newer"
            } else {
                "older"
            };
            return Err(UmweltError::public(
                UmweltErrorCode::UnsupportedFileVersion,
                format!(
                    "Umwelt file version {version} is {relation} than supported version {FILE_VERSION}."
                ),
            ));
        }
        configure_connection(&connection, Some(durability))?;
        Ok(Self {
            connection,
            provenances: ProvenanceCache::default(),
            _lock: Some(lock),
        })
    }

    pub fn is_autocommit(&self) -> bool {
        self.connection.is_autocommit()
    }

    /// Rebuilds the file to drop free pages, then truncates the WAL. Slow on
    /// large files; meant for maintenance and generated fixtures, not hot paths.
    pub fn vacuum(&mut self) -> Result<()> {
        self.connection
            .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(database_error)
    }

    /// Bytes used by each table and index, largest first (SQLite `dbstat`).
    pub fn storage_breakdown(&self) -> Result<Vec<(String, u64)>> {
        let mut statement = self
            .connection
            .prepare("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name ORDER BY 2 DESC")
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(database_error)?;
        rows.map(|row| {
            let (name, bytes) = row.map_err(database_error)?;
            Ok((name, to_u64(bytes)?))
        })
        .collect()
    }

    /// The `synchronous` pragma currently in effect (0 OFF, 1 NORMAL, 2 FULL).
    pub fn synchronous_level(&self) -> Result<i64> {
        self.connection
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .map_err(database_error)
    }

    pub fn explain_record_query(&self, query: &RecordQuery) -> Result<Vec<String>> {
        let Some(keys) = KeyBlobs::from_query(query) else {
            return Ok(Vec::new());
        };
        let current = query.ancestors.is_empty() && query.axis.is_none();
        let sql = format!("EXPLAIN QUERY PLAN {}", build_claim_query(query, current));
        query_plan(&self.connection, &sql, query, &keys)
    }

    pub fn explain_withheld_count(&self, query: &RecordQuery) -> Result<Vec<String>> {
        let Some(keys) = KeyBlobs::from_query(query) else {
            return Ok(Vec::new());
        };
        let current = query.ancestors.is_empty() && query.axis.is_none();
        let sql = format!(
            "EXPLAIN QUERY PLAN {}",
            build_withheld_count(query, current)
        );
        query_plan(&self.connection, &sql, query, &keys)
    }

    pub fn explain_latest_holding(&self) -> Result<Vec<String>> {
        let key = vec![0_u8; rows::STORED_KEY_BYTES];
        let mut statement = self
            .connection
            .prepare_cached(&format!("EXPLAIN QUERY PLAN {LATEST_HOLDING_CURRENT}"))
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params!["project", "holder", key, i64::MAX, i64::MAX],
                |row| row.get::<_, String>(3),
            )
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    /// Plan of `latest_holding` when reading a draft (always the historical
    /// path), here one nested `MAX_DRAFT_DEPTH` deep.
    pub fn explain_latest_holding_in_draft(&self) -> Result<Vec<String>> {
        let key = vec![0_u8; rows::STORED_KEY_BYTES];
        let scope = ScopeBinding::of(&deepest_scope())?;
        let mut parameters: Vec<(&str, &dyn ToSql)> = vec![
            (":project_id", &"project"),
            (":holder", &"holder"),
            (":claim_key", &key),
            (":tick", &i64::MAX),
            (":version", &i64::MAX),
        ];
        parameters.extend(scope.parameters());
        let mut statement = self
            .connection
            .prepare_cached(&format!("EXPLAIN QUERY PLAN {LATEST_HOLDING_HISTORICAL}"))
            .map_err(database_error)?;
        let rows = statement
            .query_map(&parameters[..], |row| row.get::<_, String>(3))
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    /// Plans of every statement promote and purge run, in order.
    pub fn explain_draft_maintenance(&self) -> Result<Vec<(String, Vec<String>)>> {
        PROMOTE_STATEMENTS
            .iter()
            .chain(PURGE_EMBEDDINGS.iter())
            .chain(PURGE_STATEMENTS.iter())
            .chain([
                &PURGE_CHANGE_FEED,
                &PURGE_TRANSITIONS,
                &PURGE_RECORDS,
                &PURGE_EVENTS,
                &PURGE_ATTEMPTS,
            ])
            .map(|sql| {
                let mut statement = self
                    .connection
                    .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                    .map_err(database_error)?;
                // Promote statements also bind the parent.
                let rows = if statement.parameter_count() == 3 {
                    statement.query_map(params!["project", "draft", "parent"], plan_detail)
                } else {
                    statement.query_map(params!["project", "draft"], plan_detail)
                }
                .map_err(database_error)?;
                let plan = rows
                    .map(|row| row.map_err(database_error))
                    .collect::<Result<Vec<_>>>()?;
                Ok(((*sql).to_owned(), plan))
            })
            .collect()
    }

    /// Plan of an event page in main (`draft` false) or in a draft nested
    /// `MAX_DRAFT_DEPTH` deep.
    pub fn explain_event_page(&self, draft: bool) -> Result<Vec<String>> {
        let no_cursor: Option<i64> = None;
        let no_id: Option<String> = None;
        let scope = ScopeBinding::of(&deepest_scope())?;
        let parameters: Vec<(&str, &dyn ToSql)> = if draft {
            let mut parameters: Vec<(&str, &dyn ToSql)> = vec![
                (":project_id", &"project"),
                (":tick", &i64::MAX),
                (":version", &i64::MAX),
                (":cursor_tick", &no_cursor),
                (":cursor_version", &no_cursor),
                (":cursor_id", &no_id),
                (":limit", &50_i64),
            ];
            parameters.extend(scope.parameters());
            parameters
        } else {
            Vec::new()
        };
        let main: [&dyn ToSql; 8] = [
            &"project",
            &"main",
            &i64::MAX,
            &i64::MAX,
            &no_cursor,
            &no_cursor,
            &no_id,
            &50_i64,
        ];
        let sql = if draft { EVENTS_DRAFT } else { EVENTS_MAIN };
        let mut statement = self
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .map_err(database_error)?;
        let rows = if draft {
            statement.query_map(&parameters[..], plan_detail)
        } else {
            statement.query_map(&main[..], plan_detail)
        }
        .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    /// Plan of an event page on an additional axis, in a draft nested
    /// `MAX_DRAFT_DEPTH` deep.
    pub fn explain_event_page_on_axis(&self) -> Result<Vec<String>> {
        let no_cursor: Option<i64> = None;
        let no_id: Option<String> = None;
        let scope = ScopeBinding::new(&deepest_scope().timeline_id, &deepest_scope().ancestors)?;
        let parameters = scope.with(&[
            (":project_id", &"project"),
            (":axis", &"reading"),
            (":tick", &i64::MAX),
            (":version", &i64::MAX),
            (":cursor_tick", &no_cursor),
            (":cursor_version", &no_cursor),
            (":cursor_id", &no_id),
            (":limit", &50_i64),
        ]);
        let mut statement = self
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {EVENTS_ON_AXIS}"))
            .map_err(database_error)?;
        let rows = statement
            .query_map(&parameters[..], plan_detail)
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    /// Plan of `latest_holding` on an additional axis (always the historical
    /// path), here in main.
    pub fn explain_latest_holding_on_axis(&self) -> Result<Vec<String>> {
        let key = vec![0_u8; rows::STORED_KEY_BYTES];
        let scope = ScopeBinding::of(&TimelineScope::main().on_axis(Some("reading".to_owned())))?;
        let parameters = scope.with(&[
            (":project_id", &"project"),
            (":holder", &"holder"),
            (":claim_key", &key),
            (":tick", &i64::MAX),
            (":version", &i64::MAX),
        ]);
        let sql = bound_on_axis(LATEST_HOLDING_HISTORICAL, true);
        let mut statement = self
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .map_err(database_error)?;
        let rows = statement
            .query_map(&parameters[..], plan_detail)
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    /// Plan of the vector scan of a semantic search: with a holder (from the
    /// holder's Holdings) or without one (the space's vectors).
    pub fn explain_semantic_search(&self, holder: bool) -> Result<Vec<String>> {
        let sql = if holder {
            SEMANTIC_HOLDER
        } else {
            SEMANTIC_NARRATOR
        };
        self.explain_unbound(sql)
    }

    /// Plan of the `dispositions` read, on the main axis or an additional one.
    pub fn explain_dispositions(&self, axis: bool) -> Result<Vec<String>> {
        self.explain_unbound(&bound_on_axis(DISPOSITIONS, axis))
    }

    /// Plan of `embedding.pending`.
    pub fn explain_pending_embeddings(&self) -> Result<Vec<String>> {
        self.explain_unbound(PENDING_EMBEDDINGS)
    }

    fn explain_unbound(&self, sql: &str) -> Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .map_err(database_error)?;
        let unbound = vec![rusqlite::types::Null; statement.parameter_count()];
        let rows = statement
            .query_map(rusqlite::params_from_iter(unbound), plan_detail)
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    /// Plans of the secret, told-from and witnessed lookups, by name.
    pub fn explain_knowledge_lookups(&self) -> Result<Vec<(&'static str, String)>> {
        [
            ("secrets", SECRETS),
            ("secrets of holder", SECRETS_OF_HOLDER),
            ("told from", TOLD_FROM),
            ("witnessed by", WITNESSED_BY),
        ]
        .into_iter()
        .map(|(name, sql)| {
            let mut statement = self
                .connection
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .map_err(database_error)?;
            let unbound = vec![rusqlite::types::Null; statement.parameter_count()];
            let rows = statement
                .query_map(rusqlite::params_from_iter(unbound), |row| {
                    row.get::<_, String>(3)
                })
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?;
            Ok((name, rows.join("\n")))
        })
        .collect()
    }

    pub fn explain_current_window(&self) -> Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare_cached(
                "EXPLAIN QUERY PLAN SELECT version, max_tick FROM projects WHERE id = ?1",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(["project"], |row| row.get::<_, String>(3))
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }
}

/// The detail column of an `EXPLAIN QUERY PLAN` row.
fn plan_detail(row: &rusqlite::Row<'_>) -> rusqlite::Result<String> {
    row.get(3)
}

const LATEST_HOLDING_CURRENT: &str = "SELECT r.id, r.attitude, r.polarity, r.confidence \
     FROM records r \
     WHERE r.project_id = ?1 AND r.holder_id = ?2 AND r.claim_key = ?3 \
     AND r.timeline_id = 'main' AND r.kind = 'holding' \
     AND (r.accessibility IS NULL OR r.accessibility = 'available') \
     AND r.tick <= ?4 AND r.recorded_version <= ?5 \
     AND NOT EXISTS (SELECT 1 FROM current_status s \
     WHERE s.project_id = r.project_id AND s.timeline_id = 'main' AND s.record_id = r.id) \
     ORDER BY r.tick DESC, r.recorded_version DESC, r.id DESC LIMIT 1";
const LATEST_HOLDING_HISTORICAL: &str = concat!(
    "SELECT r.id, r.attitude, r.polarity, r.confidence \
     FROM records r \
     WHERE r.project_id = :project_id AND r.holder_id = :holder AND r.claim_key = :claim_key \
     AND ",
    visible!("r"),
    " AND r.kind = 'holding' \
     AND (r.accessibility IS NULL OR r.accessibility = 'available') \
     AND r.tick <= :tick AND r.recorded_version <= :version \
     AND NOT EXISTS (SELECT 1 FROM transitions t \
     WHERE t.project_id = r.project_id AND t.target_id = r.id \
     AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    ") ORDER BY r.tick DESC, r.recorded_version DESC, r.id DESC LIMIT 1"
);

/// Claim and slot keys travel as 64-char hex and are stored as 16-byte BLOBs
/// (see `rows::key_blob`).
struct KeyBlobs {
    claim_key: Option<Vec<u8>>,
    slot_key: Option<Vec<u8>>,
}

impl KeyBlobs {
    fn from_query(query: &RecordQuery) -> Option<Self> {
        let claim_key = match &query.claim_key {
            Some(value) => Some(key_blob(value)?),
            None => None,
        };
        let slot_key = match &query.slot_key {
            Some(value) => Some(key_blob(value)?),
            None => None,
        };
        Some(Self {
            claim_key,
            slot_key,
        })
    }
}

/// Filters shared by a page query and its withheld count.
///
/// When a holder is given, `kind` is only added if no narrower holder filter is
/// present: every row with a holder is a Holding, and an extra equality on
/// `kind` would let the planner pick the holder-wide index instead of the
/// selective one.
fn push_claim_filters(sql: &mut String, query: &RecordQuery, skip_about: bool) {
    push_visible(sql, "r", !query.ancestors.is_empty());
    if query.holder_id.is_none() && query.subject_id.is_some() && query.predicate.is_some() {
        sql.push_str(" AND r.kind = 'fact' AND r.holder_id IS NULL");
    }
    if query.holder_id.is_some() {
        sql.push_str(" AND r.holder_id = :holder_id");
    }
    let narrower_holder_filter = query.claim_key.is_some()
        || query.subject_id.is_some()
        || query.predicate.is_some()
        || query.object_entity_id.is_some()
        || query.about_entity_id.is_some();
    if query.record_kind.is_some() && (query.holder_id.is_none() || !narrower_holder_filter) {
        sql.push_str(" AND r.kind = :record_kind");
    }
    if query.claim_key.is_some() {
        sql.push_str(" AND r.claim_key = :claim_key");
    }
    if query.slot_key.is_some() {
        sql.push_str(" AND r.slot_key = :slot_key");
    }
    if query.subject_id.is_some() {
        sql.push_str(" AND r.subject_id = :subject_id");
    }
    if query.predicate.is_some() {
        sql.push_str(" AND r.predicate = :predicate");
    }
    if query.object_entity_id.is_some() {
        sql.push_str(" AND r.object_entity_id = :object_entity_id");
    }
    if query.about_entity_id.is_some() && !skip_about {
        sql.push_str(
            " AND (r.subject_id = :about_entity_id OR r.object_entity_id = :about_entity_id)",
        );
    }
    let attitude_parameters = attitude_parameters(&query.attitudes);
    if !attitude_parameters.is_empty() {
        sql.push_str(" AND r.attitude IN (");
        sql.push_str(
            &attitude_parameters
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", "),
        );
        sql.push(')');
    }
    if query.minimum_confidence.is_some() {
        sql.push_str(" AND r.confidence >= :minimum_confidence");
    }
    sql.push_str(" AND r.tick <= :tick AND r.recorded_version <= :as_of_version");
}

/// Adds the timeline visibility of `alias` (`r` or `t`). Main reads keep a
/// plain equality; drafts also see their ancestors' rows (`visible!`).
fn push_visible(sql: &mut String, alias: &str, draft: bool) {
    match (draft, alias) {
        (true, "t") => {
            sql.push_str(" AND ");
            sql.push_str(visible!("t"));
        }
        (true, _) => {
            sql.push_str(" AND ");
            sql.push_str(visible!("r"));
        }
        (false, _) => sql.push_str(&format!(" AND {alias}.timeline_id = :timeline_id")),
    }
}

/// `current` (main, at its head window) uses the materialized status; every
/// other read, including every draft read, checks transitions.
fn push_active_filter(sql: &mut String, current: bool, draft: bool) {
    if current {
        sql.push_str(
            " AND NOT EXISTS (SELECT 1 FROM current_status s \
             WHERE s.project_id = r.project_id AND s.timeline_id = 'main' \
             AND s.record_id = r.id)",
        );
    } else {
        sql.push_str(
            " AND NOT EXISTS (SELECT 1 FROM transitions t \
             WHERE t.project_id = r.project_id AND t.target_id = r.id \
             AND t.tick <= :tick AND t.recorded_version <= :as_of_version",
        );
        push_visible(sql, "t", draft);
        sql.push(')');
    }
}

const BRANCH_SELECT: &str = "SELECT r.id AS id, r.tick AS tick, \
     r.recorded_version AS recorded_version FROM records r WHERE r.project_id = :project_id";
const PAGE_SELECT: &str = concat!(
    "SELECT ",
    record_columns!(),
    " FROM records r WHERE r.project_id = :project_id"
);

/// One ordered, index-backed scan of the record table.
fn claim_branch(
    select: &str,
    query: &RecordQuery,
    current: bool,
    about_column: Option<&str>,
) -> String {
    let mut sql = String::from(select);
    push_claim_filters(&mut sql, query, about_column.is_some());
    match about_column {
        Some("subject") => sql.push_str(" AND r.subject_id = :about_entity_id"),
        Some(_) => sql.push_str(
            " AND r.object_entity_id = :about_entity_id AND r.subject_id <> :about_entity_id",
        ),
        None => {}
    }
    if query.include_withheld {
        // Withheld rows are included on request.
    } else {
        sql.push_str(" AND (r.accessibility IS NULL OR r.accessibility = 'available')");
    }
    if query.cursor.is_some() {
        sql.push_str(
            " AND (r.tick, r.recorded_version, r.id) > \
             (:cursor_tick, :cursor_version, :cursor_id)",
        );
    }
    push_active_filter(&mut sql, current, !query.ancestors.is_empty());
    sql.push_str(" ORDER BY r.tick, r.recorded_version, r.id LIMIT :limit");
    sql
}

/// Page query over claims. The order always comes from an index; `about`
/// becomes two index-backed branches (subject and object) merged under a
/// final bounded sort of at most `2 * limit` rows.
fn build_claim_query(query: &RecordQuery, current: bool) -> String {
    let sql = build_claim_query_on_main_axis(query, current);
    bound_on_axis(&sql, query.axis.is_some()).into_owned()
}

fn build_claim_query_on_main_axis(query: &RecordQuery, current: bool) -> String {
    if query.about_entity_id.is_none() {
        return claim_branch(PAGE_SELECT, query, current, None);
    }
    format!(
        "SELECT {RECORD_COLUMNS} FROM \
         (SELECT * FROM ({}) UNION ALL SELECT * FROM ({})) c \
         JOIN records r ON r.project_id = :project_id AND r.id = c.id \
         ORDER BY c.tick, c.recorded_version, c.id LIMIT :limit",
        claim_branch(BRANCH_SELECT, query, current, Some("subject")),
        claim_branch(BRANCH_SELECT, query, current, Some("object"))
    )
}

/// Counts withheld Holdings through the partial `claims_withheld` index, so
/// the cost follows the number of withheld rows, not the holder's history.
fn build_withheld_count(query: &RecordQuery, current: bool) -> String {
    let mut sql = String::from(
        "SELECT COUNT(*) FROM records r INDEXED BY claims_withheld \
         WHERE r.project_id = :project_id \
         AND r.accessibility IN ('suppressed', 'inaccessible')",
    );
    push_claim_filters(&mut sql, query, false);
    push_active_filter(&mut sql, current, !query.ancestors.is_empty());
    bound_on_axis(&sql, query.axis.is_some()).into_owned()
}

fn attitude_parameters(attitudes: &[Attitude]) -> Vec<(&'static str, &'static dyn ToSql)> {
    static KNOWS: &str = "knows";
    static BELIEVES: &str = "believes";
    static SUSPECTS: &str = "suspects";
    static REMEMBERS: &str = "remembers";
    let mut parameters: Vec<(&str, &dyn ToSql)> = Vec::new();
    for (attitude, name, value) in [
        (Attitude::Knows, ":attitude_knows", &KNOWS as &dyn ToSql),
        (
            Attitude::Believes,
            ":attitude_believes",
            &BELIEVES as &dyn ToSql,
        ),
        (
            Attitude::Suspects,
            ":attitude_suspects",
            &SUSPECTS as &dyn ToSql,
        ),
        (
            Attitude::Remembers,
            ":attitude_remembers",
            &REMEMBERS as &dyn ToSql,
        ),
    ] {
        if attitudes.contains(&attitude) {
            parameters.push((name, value));
        }
    }
    parameters
}

/// Binds exactly the named parameters the generated SQL uses.
#[allow(clippy::too_many_arguments)]
fn claim_parameters<'a>(
    sql: &str,
    query: &'a RecordQuery,
    keys: &'a KeyBlobs,
    as_of_version: &'a i64,
    scope: &'a ScopeBinding,
    cursor_version: &'a Option<i64>,
    limit: &'a i64,
) -> Vec<(&'static str, &'a dyn ToSql)> {
    let mut parameters: Vec<(&str, &dyn ToSql)> = vec![(":project_id", &query.project_id)];
    if sql.contains(":a1") {
        parameters.extend(scope.parameters());
    } else {
        parameters.push((":timeline_id", &scope.timeline_id));
    }
    parameters.extend(scope.axis_parameter());
    if let Some(value) = &query.holder_id {
        parameters.push((":holder_id", value));
    }
    if let Some(value) = &query.record_kind {
        if sql.contains(":record_kind") {
            parameters.push((":record_kind", value));
        }
    }
    if let Some(value) = &keys.claim_key {
        parameters.push((":claim_key", value));
    }
    if let Some(value) = &keys.slot_key {
        parameters.push((":slot_key", value));
    }
    if let Some(value) = &query.subject_id {
        parameters.push((":subject_id", value));
    }
    if let Some(value) = &query.predicate {
        parameters.push((":predicate", value));
    }
    if let Some(value) = &query.object_entity_id {
        parameters.push((":object_entity_id", value));
    }
    if let Some(value) = &query.about_entity_id {
        parameters.push((":about_entity_id", value));
    }
    parameters.extend(attitude_parameters(&query.attitudes));
    if let Some(value) = &query.minimum_confidence {
        parameters.push((":minimum_confidence", value));
    }
    parameters.push((":tick", &query.tick));
    parameters.push((":as_of_version", as_of_version));
    if sql.contains(":cursor_tick") {
        if let (Some(cursor), Some(version)) = (&query.cursor, cursor_version) {
            parameters.push((":cursor_tick", &cursor.tick));
            parameters.push((":cursor_version", version));
            parameters.push((":cursor_id", &cursor.record_id));
        }
    }
    if sql.contains(":limit") {
        parameters.push((":limit", limit));
    }
    parameters
}

fn query_plan(
    connection: &Connection,
    sql: &str,
    query: &RecordQuery,
    keys: &KeyBlobs,
) -> Result<Vec<String>> {
    let as_of_version = to_i64(query.as_of_version)?;
    let scope =
        ScopeBinding::new(&query.timeline_id, &query.ancestors)?.on_axis(query.axis.as_deref());
    let cursor_version = query
        .cursor
        .as_ref()
        .map(|value| to_i64(value.recorded_version))
        .transpose()?;
    let limit = i64::from(query.limit.clamp(1, 1000)) + 1;
    let parameters = claim_parameters(
        sql,
        query,
        keys,
        &as_of_version,
        &scope,
        &cursor_version,
        &limit,
    );
    let mut statement = connection.prepare_cached(sql).map_err(database_error)?;
    let rows = statement
        .query_map(&parameters[..], |row| row.get::<_, String>(3))
        .map_err(database_error)?;
    rows.map(|row| row.map_err(database_error)).collect()
}

fn validate_path(path: &Path) -> Result<PathBuf> {
    if path.extension().and_then(|value| value.to_str()) != Some("umw") {
        return Err(UmweltError::validation(
            "Umwelt database files must use the .umw extension.",
        ));
    }
    Ok(path.to_path_buf())
}

fn acquire_lock(path: &Path) -> Result<File> {
    let mut lock_name = OsString::from(path.as_os_str());
    lock_name.push(".lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(PathBuf::from(lock_name))
        .map_err(|source| {
            UmweltError::internal(
                UmweltErrorCode::DatabaseError,
                "Could not open the Umwelt lock file.",
                source,
            )
        })?;
    if !lock.try_lock_exclusive().map_err(|source| {
        UmweltError::internal(
            UmweltErrorCode::DatabaseError,
            "Could not lock the Umwelt file.",
            source,
        )
    })? {
        return Err(UmweltError::public(
            UmweltErrorCode::FileLocked,
            "The Umwelt file is already open by another process.",
        ));
    }
    Ok(lock)
}

/// `durability` is `None` for in-memory stores, which have no journal to tune.
fn configure_connection(connection: &Connection, durability: Option<Durability>) -> Result<()> {
    // rusqlite caches 16 statements by default. One commit alone runs more
    // than 20 distinct statements (inserts plus validation lookups), so the
    // default cache evicts everything and every call recompiles its SQL.
    connection.set_prepared_statement_cache_capacity(256);
    // The stored text hash of a document's text, so that pending vectors are
    // found in SQL (see `PENDING_EMBEDDINGS`).
    connection
        .create_scalar_function(
            "umwelt_text_hash",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |context| Ok(rows::stored_text_hash(&context.get::<String>(0)?)),
        )
        .map_err(database_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(database_error)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(database_error)?;
    if let Some(durability) = durability {
        // SQLite's default page cache is ~2 MB. A commit touches a dozen
        // B-trees spread across the file; with the default cache nearly every
        // index insert becomes a page read. 64 MiB keeps the hot pages resident.
        connection
            .pragma_update(None, "cache_size", -65_536)
            .map_err(database_error)?;
        // Reads of pages already in the OS cache skip a copy and a syscall.
        // The window is address space, not memory: 2 GiB (SQLite's default
        // ceiling) covers whole files, so scattered reads such as the vectors
        // of a semantic search stay mapped.
        connection
            .pragma_update(None, "mmap_size", 2_147_418_112_i64)
            .map_err(database_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(database_error)?;
        connection
            .pragma_update(None, "synchronous", durability.pragma_value())
            .map_err(database_error)?;
    }
    Ok(())
}

impl Store for SqliteStore {
    fn create_project(&mut self, project: &Project) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        insert_project(&transaction, project)?;
        transaction.commit().map_err(database_error)
    }

    fn project(&self, project_id: &str) -> Result<Project> {
        project_from_connection(&self.connection, project_id)?.ok_or_else(|| {
            UmweltError::not_found(format!("Project '{project_id}' does not exist."))
        })
    }

    fn projects(&self) -> Result<Vec<Project>> {
        let mut statement = self.connection.prepare_cached(
            "SELECT id, name, description, timescale, vocabulary_mode, created_at, version FROM projects ORDER BY id",
        ).map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            })
            .map_err(database_error)?;
        rows.map(|row| {
            let (id, name, description, timescale, vocabulary, created_at, version) =
                row.map_err(database_error)?;
            Ok(Project {
                id,
                name,
                description,
                timescale,
                vocabulary_mode: from_json(&format!("\"{vocabulary}\""))?,
                created_at,
                version: to_u64(version)?,
            })
        })
        .collect()
    }

    fn put_entity_type(&mut self, entity_type: &EntityType) -> Result<()> {
        insert_entity_type(&self.connection, entity_type)
    }

    fn entity_types(&self, project_id: &str) -> Result<Vec<EntityType>> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT payload FROM entity_types WHERE project_id = ?1 ORDER BY id")
            .map_err(database_error)?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        rows.map(|row| from_json(&row.map_err(database_error)?))
            .collect()
    }

    fn put_entity(&mut self, entity: &Entity) -> Result<()> {
        insert_entity(&self.connection, entity)
    }

    fn update_entity(&mut self, entity: &Entity) -> Result<()> {
        replace_entity(&self.connection, entity)
    }

    fn write_entities(&mut self, writes: &[EntityWrite]) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        for write in writes {
            match write {
                EntityWrite::Create(entity) => insert_entity(&transaction, entity)?,
                EntityWrite::Update(entity) => replace_entity(&transaction, entity)?,
            }
        }
        transaction.commit().map_err(database_error)
    }

    fn entity_by_external(
        &self,
        project_id: &str,
        reference: &ExternalRef,
    ) -> Result<Option<Entity>> {
        query_json_optional(
            &self.connection,
            "SELECT payload FROM entities WHERE project_id = ?1 AND external_system = ?2 AND external_id = ?3",
            params![project_id, reference.system, reference.id],
        )
    }

    fn put_predicate(&mut self, project_id: &str, predicate: &PredicateDef) -> Result<()> {
        insert_predicate(&self.connection, project_id, predicate)
    }

    fn write_world(&mut self, write: &WorldWrite) -> Result<()> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if let Some(project) = &write.project {
            insert_project(&transaction, project)?;
        }
        for entity_type in &write.entity_types {
            insert_entity_type(&transaction, entity_type)?;
        }
        for entity in &write.entities {
            insert_entity(&transaction, entity)?;
        }
        for predicate in &write.predicates {
            insert_predicate(&transaction, &write.project_id, predicate)?;
        }
        for axis in &write.axes {
            transaction
                .execute(
                    "INSERT INTO axes (project_id, id, label) VALUES (?1, ?2, ?3)",
                    params![write.project_id, axis.id, axis.label],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }

    fn define_axis(&mut self, project_id: &str, axis: &AxisDef) -> Result<AxisDef> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        project_from_connection(&transaction, project_id)?.ok_or_else(|| {
            UmweltError::not_found(format!("Project '{project_id}' does not exist."))
        })?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT label FROM axes WHERE project_id = ?1 AND id = ?2",
                params![project_id, axis.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        match existing {
            Some(label) if label == axis.label => {}
            Some(label) => {
                return Err(UmweltError::public(
                    UmweltErrorCode::Conflict,
                    format!(
                        "Axis '{}' is already defined with the label '{label}'.",
                        axis.id
                    ),
                )
                .with_details(json!({ "axis": { "id": axis.id, "label": label } })));
            }
            None => {
                transaction
                    .execute(
                        "INSERT INTO axes (project_id, id, label) VALUES (?1, ?2, ?3)",
                        params![project_id, axis.id, axis.label],
                    )
                    .map_err(database_error)?;
            }
        }
        transaction.commit().map_err(database_error)?;
        Ok(axis.clone())
    }

    fn axes(&self, project_id: &str) -> Result<Vec<AxisDef>> {
        self.project(project_id)?;
        let mut statement = self
            .connection
            .prepare_cached("SELECT id, label FROM axes WHERE project_id = ?1 ORDER BY id")
            .map_err(database_error)?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok(AxisDef {
                    id: row.get(0)?,
                    label: row.get(1)?,
                })
            })
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    fn axis_max_tick(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        axis: &str,
    ) -> Result<Option<i64>> {
        let scope = ScopeBinding::new(&scope.timeline_id, &scope.ancestors)?;
        let parameters = scope.with(&[(":project_id", &project_id), (":axis", &axis)]);
        self.connection
            .prepare_cached(AXIS_MAX_TICK)
            .map_err(database_error)?
            .query_row(&parameters[..], |row| row.get(0))
            .optional()
            .map_err(database_error)
    }

    fn define_embedding_space(
        &mut self,
        project_id: &str,
        space: &EmbeddingSpace,
    ) -> Result<EmbeddingSpace> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        project_from_connection(&transaction, project_id)?.ok_or_else(|| {
            UmweltError::not_found(format!("Project '{project_id}' does not exist."))
        })?;
        match embedding_space(&transaction, project_id, &space.id)? {
            Some(existing) if existing == *space => {}
            Some(existing) => {
                return Err(UmweltError::public(
                    UmweltErrorCode::Conflict,
                    format!(
                        "Embedding space '{}' is already defined differently.",
                        space.id
                    ),
                )
                .with_details(json!({ "space": existing })));
            }
            None => {
                transaction
                    .execute(
                        "INSERT INTO embedding_spaces (project_id, id, dimensions, model) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![project_id, space.id, space.dimensions, space.model],
                    )
                    .map_err(database_error)?;
            }
        }
        transaction.commit().map_err(database_error)?;
        Ok(space.clone())
    }

    fn embedding_spaces(&self, project_id: &str) -> Result<Vec<EmbeddingSpace>> {
        self.project(project_id)?;
        let mut statement = self
            .connection
            .prepare_cached(
                "SELECT id, dimensions, model FROM embedding_spaces WHERE project_id = ?1 ORDER BY id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([project_id], |row| {
                Ok(EmbeddingSpace {
                    id: row.get(0)?,
                    dimensions: row.get(1)?,
                    model: row.get(2)?,
                })
            })
            .map_err(database_error)?;
        rows.map(|row| row.map_err(database_error)).collect()
    }

    fn drop_embedding_space(&mut self, project_id: &str, space_id: &str) -> Result<u64> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        require_embedding_space(&transaction, project_id, space_id)?;
        let removed = transaction
            .execute(
                "DELETE FROM embeddings WHERE project_id = ?1 AND space_id = ?2",
                params![project_id, space_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM embedding_spaces WHERE project_id = ?1 AND id = ?2",
                params![project_id, space_id],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        u64::try_from(removed).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Row count overflowed.")
        })
    }

    fn put_embeddings(
        &mut self,
        project_id: &str,
        space_id: &str,
        items: &[EmbeddingItem],
    ) -> Result<PutEmbeddingsResult> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let space = require_embedding_space(&transaction, project_id, space_id)?;
        let mut stale = 0_u64;
        for item in items {
            if item.vector.len() != space.dimensions as usize {
                return Err(UmweltError::validation(format!(
                    "The vector of {} '{}' has {} dimensions; the space has {}.",
                    item.doc_type,
                    item.doc_id,
                    item.vector.len(),
                    space.dimensions
                )));
            }
            let text = document_text(
                &transaction,
                &self.provenances,
                project_id,
                &item.doc_type,
                &item.doc_id,
            )?
            .ok_or_else(|| {
                UmweltError::validation(format!(
                    "{} '{}' is not searchable and cannot have a vector.",
                    item.doc_type, item.doc_id
                ))
            })?;
            if text_hash(&text) != item.text_hash.to_ascii_lowercase() {
                stale += 1;
            }
            let stored_hash = rows::key_blob(&item.text_hash.to_ascii_lowercase())
                .ok_or_else(|| UmweltError::validation("textHash must be 64 hex characters."))?;
            execute_cached(
                &transaction,
                "INSERT INTO embeddings (project_id, space_id, doc_type, doc_id, text_hash, vector) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(project_id, space_id, doc_type, doc_id) DO UPDATE SET \
                 text_hash = excluded.text_hash, vector = excluded.vector",
                params![
                    project_id,
                    space_id,
                    item.doc_type,
                    item.doc_id,
                    stored_hash,
                    rows::vector_blob(&item.vector)
                ],
            )?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(PutEmbeddingsResult {
            stored: u64::try_from(items.len()).unwrap_or(u64::MAX),
            stale,
        })
    }

    fn delete_embeddings(
        &mut self,
        project_id: &str,
        space_id: &str,
        items: &[EmbeddingDocRef],
    ) -> Result<u64> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        require_embedding_space(&transaction, project_id, space_id)?;
        let mut removed = 0_u64;
        for item in items {
            removed += transaction
                .prepare_cached(
                    "DELETE FROM embeddings WHERE project_id = ?1 AND space_id = ?2 \
                     AND doc_type = ?3 AND doc_id = ?4",
                )
                .map_err(database_error)?
                .execute(params![project_id, space_id, item.doc_type, item.doc_id])
                .map_err(database_error)? as u64;
        }
        transaction.commit().map_err(database_error)?;
        Ok(removed)
    }

    fn pending_embeddings(&self, query: &PendingQuery) -> Result<PendingPage> {
        require_embedding_space(&self.connection, &query.project_id, &query.space_id)?;
        pending_embeddings(&self.connection, query)
    }

    fn put_evidence(&mut self, evidence: &EvidenceSource) -> Result<()> {
        self.project(&evidence.project_id)?;
        self.connection
            .execute(
                "INSERT INTO evidence_sources (project_id, id, payload) VALUES (?1, ?2, ?3)",
                params![evidence.project_id, evidence.id, to_json(evidence)?],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn evidence(&self, project_id: &str, evidence_id: &str) -> Result<EvidenceSource> {
        query_json_optional(
            &self.connection,
            "SELECT payload FROM evidence_sources WHERE project_id = ?1 AND id = ?2",
            params![project_id, evidence_id],
        )?
        .ok_or_else(|| {
            UmweltError::not_found(format!("Evidence source '{evidence_id}' does not exist."))
        })
    }

    fn commit_batch(&mut self, commits: &[PreparedCommit]) -> Result<Vec<CommitOutcome>> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let mut outcomes = Vec::with_capacity(commits.len());
        let caches = WriteCaches::default();
        // The target is read once per timeline; each commit then moves its
        // versions forward in memory, and they are stored once.
        let mut target: Option<WriteTarget> = None;
        for prepared in commits {
            let current = match target.take() {
                Some(known) if known.scope.timeline_id == prepared.input.timeline() => known,
                other => {
                    if let Some(previous) = other {
                        store_head(&transaction, &prepared.input.project_id, &previous)?;
                    }
                    write_target(&transaction, prepared)?
                }
            };
            if let Some(outcome) = replay_outcome(&transaction, prepared, &current.scope)? {
                outcomes.push(outcome);
                target = Some(current);
                continue;
            }
            let committed =
                commit_new(&transaction, prepared, &current, &caches, &self.provenances)?;
            target = Some(current.advanced(&committed));
            outcomes.push(committed.outcome);
        }
        if let (Some(target), Some(first)) = (&target, commits.first()) {
            store_head(&transaction, &first.input.project_id, target)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(outcomes)
    }

    fn apply_proposal(
        &mut self,
        prepared: &PreparedCommit,
        proposal_id: &str,
        status: ProposalStatus,
    ) -> Result<CommitOutcome> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let target = write_target(&transaction, prepared)?;
        if let Some(outcome) = replay_outcome(&transaction, prepared, &target.scope)? {
            transaction.commit().map_err(database_error)?;
            return Ok(outcome);
        }
        if prepared.input.records.is_empty() && prepared.input.transitions.is_empty() {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                "A proposal must have at least one accepted operation before it can be applied.",
            ));
        }
        let mut proposal =
            proposal_from_connection(&transaction, proposal_id)?.ok_or_else(|| {
                UmweltError::not_found(format!("Proposal '{proposal_id}' does not exist."))
            })?;
        if proposal.status != ProposalStatus::Pending {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                "Only pending proposals can be applied.",
            ));
        }
        let committed = commit_new(
            &transaction,
            prepared,
            &target,
            &WriteCaches::default(),
            &self.provenances,
        )?;
        store_head(
            &transaction,
            &prepared.input.project_id,
            &target.advanced(&committed),
        )?;
        proposal.status = status;
        update_proposal(&transaction, &proposal)?;
        transaction.commit().map_err(database_error)?;
        Ok(committed.outcome)
    }

    fn timeline(&self, project_id: &str, timeline_id: &str) -> Result<Timeline> {
        timeline_from_connection(&self.connection, project_id, timeline_id)?.ok_or_else(|| {
            UmweltError::not_found(format!("Timeline '{timeline_id}' does not exist."))
        })
    }

    fn timelines(&self, project_id: &str, status: Option<TimelineStatus>) -> Result<Vec<Timeline>> {
        let main = timeline_from_connection(&self.connection, project_id, MAIN_TIMELINE_ID)?
            .ok_or_else(|| {
                UmweltError::not_found(format!("Project '{project_id}' does not exist."))
            })?;
        let mut statement = self
            .connection
            .prepare_cached(&format!(
                "SELECT {TIMELINE_COLUMNS} FROM timelines WHERE project_id = ?1 \
                 AND (?2 IS NULL OR status = ?2) ORDER BY created_at, id"
            ))
            .map_err(database_error)?;
        let status_filter = status.as_ref().map(enum_name).transpose()?;
        let rows = statement
            .query_map(params![project_id, status_filter], timeline_row)
            .map_err(database_error)?;
        let mut timelines = Vec::new();
        if status.is_none_or(|value| value == main.status) {
            timelines.push(main);
        }
        for row in rows {
            timelines.push(timeline_from_row(
                &self.connection,
                project_id,
                row.map_err(database_error)?,
            )?);
        }
        Ok(timelines)
    }

    fn timeline_scope(&self, project_id: &str, timeline: &Timeline) -> Result<TimelineScope> {
        if timeline.project_id != project_id {
            return Err(UmweltError::not_found(format!(
                "Timeline '{}' does not exist.",
                timeline.id
            )));
        }
        scope_of(&self.connection, timeline)
    }

    fn open_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        parent_id: &str,
        label: Option<&str>,
        created_at: &str,
    ) -> Result<Timeline> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let parent = timeline_from_connection(&transaction, project_id, parent_id)?
            .filter(|parent| parent.purged_at.is_none())
            .ok_or_else(|| {
                UmweltError::not_found(format!("Timeline '{parent_id}' does not exist."))
            })?;
        require_active(&parent)?;
        if parent.depth >= MAX_DRAFT_DEPTH {
            return Err(UmweltError::validation(format!(
                "Drafts can be nested at most {MAX_DRAFT_DEPTH} deep below main."
            ))
            .with_details(json!({ "parentTimelineId": parent_id, "maxDepth": MAX_DRAFT_DEPTH })));
        }
        if timeline_from_connection(&transaction, project_id, timeline_id)?.is_some() {
            return Err(UmweltError::public(
                UmweltErrorCode::Conflict,
                format!("Timeline '{timeline_id}' already exists."),
            )
            .with_details(json!({ "timelineId": timeline_id })));
        }
        transaction
            .execute(
                "INSERT INTO timelines (project_id, id, parent_id, kind, status, base_version, head_version, max_tick, label, created_at) \
                 VALUES (?1, ?2, ?3, 'draft', 'active', ?4, ?4, ?5, ?6, ?7)",
                params![
                    project_id,
                    timeline_id,
                    parent_id,
                    to_i64(parent.head_version)?,
                    parent.max_tick,
                    label,
                    created_at
                ],
            )
            .map_err(database_error)?;
        let timeline = timeline_from_connection(&transaction, project_id, timeline_id)?
            .ok_or_else(|| {
                UmweltError::public(UmweltErrorCode::InvalidState, "Draft was not created.")
            })?;
        transaction.commit().map_err(database_error)?;
        Ok(timeline)
    }

    fn promote_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        closed_at: &str,
    ) -> Result<Timeline> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let draft = existing_draft(&transaction, project_id, timeline_id)?;
        require_active(&draft)?;
        let parent_id = parent_of(&draft)?;
        let parent =
            timeline_from_connection(&transaction, project_id, parent_id)?.ok_or_else(|| {
                UmweltError::not_found(format!("Timeline '{parent_id}' does not exist."))
            })?;
        require_active(&parent)?;
        let base = draft.base_version.unwrap_or_default();
        if parent.head_version != base {
            let mut details = json!({
                "baseVersion": base,
                "parentTimelineId": parent_id,
                "parentHeadVersion": parent.head_version,
            });
            if parent_id == MAIN_TIMELINE_ID {
                details["mainHeadVersion"] = json!(parent.head_version);
            }
            return Err(UmweltError::public(
                UmweltErrorCode::VersionConflict,
                format!(
                    "Timeline '{parent_id}' moved from version {base} to {} since the draft was opened.",
                    parent.head_version
                ),
            )
            .with_details(details));
        }
        for sql in PROMOTE_STATEMENTS {
            let mut statement = transaction.prepare_cached(sql).map_err(database_error)?;
            // The last statement only deletes the draft's rows: no parent.
            if statement.parameter_count() == 3 {
                statement.execute(params![project_id, timeline_id, parent_id])
            } else {
                statement.execute(params![project_id, timeline_id])
            }
            .map_err(database_error)?;
        }
        if parent_id == MAIN_TIMELINE_ID {
            transaction
                .execute(
                    "UPDATE projects SET version = ?2, max_tick = MAX(max_tick, ?3) WHERE id = ?1",
                    params![project_id, to_i64(draft.head_version)?, draft.max_tick],
                )
                .map_err(database_error)?;
        } else {
            transaction
                .execute(
                    "UPDATE timelines SET head_version = ?3, max_tick = MAX(max_tick, ?4) \
                     WHERE project_id = ?1 AND id = ?2",
                    params![
                        project_id,
                        parent_id,
                        to_i64(draft.head_version)?,
                        draft.max_tick
                    ],
                )
                .map_err(database_error)?;
        }
        // The draft's children now read its rows in the parent, up to the
        // same base versions: they see exactly what they saw before.
        let reparented = {
            let mut statement = transaction
                .prepare(
                    "SELECT id FROM timelines WHERE project_id = ?1 AND parent_id = ?2 \
                     AND purged_at IS NULL AND status <> 'promoted' ORDER BY id",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map(params![project_id, timeline_id], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(database_error)?;
            rows.map(|row| row.map_err(database_error))
                .collect::<Result<Vec<_>>>()?
        };
        transaction
            .execute(
                "UPDATE timelines SET parent_id = ?3 WHERE project_id = ?1 AND parent_id = ?2 \
                 AND purged_at IS NULL AND status <> 'promoted'",
                params![project_id, timeline_id, parent_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE timelines SET status = 'promoted', closed_at = ?3 WHERE project_id = ?1 AND id = ?2",
                params![project_id, timeline_id, closed_at],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO change_feed (project_id, timeline_id, version, type, payload) VALUES (?1, ?2, ?3, 'timeline.promoted', ?4)",
                params![
                    project_id,
                    parent_id,
                    to_i64(draft.head_version)?,
                    to_json(&json!({
                        "timelineId": timeline_id,
                        "into": parent_id,
                        "baseVersion": base,
                        "headVersion": draft.head_version,
                        "reparented": reparented,
                    }))?
                ],
            )
            .map_err(database_error)?;
        let timeline = existing_draft(&transaction, project_id, timeline_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(timeline)
    }

    fn discard_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        closed_at: &str,
        purge: bool,
    ) -> Result<Timeline> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let draft = existing_draft(&transaction, project_id, timeline_id)?;
        require_active(&draft)?;
        require_no_descendants(&transaction, project_id, timeline_id, true, "discarded")?;
        if purge {
            require_no_descendants(&transaction, project_id, timeline_id, false, "purged")?;
        }
        transaction
            .execute(
                "UPDATE timelines SET status = 'discarded', closed_at = ?3 WHERE project_id = ?1 AND id = ?2",
                params![project_id, timeline_id, closed_at],
            )
            .map_err(database_error)?;
        if purge {
            purge_rows(&transaction, project_id, timeline_id, closed_at)?;
        }
        let timeline = existing_draft(&transaction, project_id, timeline_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(timeline)
    }

    fn purge_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        purged_at: &str,
    ) -> Result<PurgeCounts> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let draft = existing_draft(&transaction, project_id, timeline_id)?;
        if draft.status != TimelineStatus::Discarded {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                "Only discarded drafts can be purged. Discard the draft first.",
            )
            .with_details(json!({ "timelineId": timeline_id, "status": draft.status })));
        }
        // Purging twice is a no-op, so a client can retry safely.
        let counts = if draft.purged_at.is_some() {
            PurgeCounts {
                events: 0,
                records: 0,
                transitions: 0,
                attempts: 0,
            }
        } else {
            require_no_descendants(&transaction, project_id, timeline_id, false, "purged")?;
            purge_rows(&transaction, project_id, timeline_id, purged_at)?
        };
        transaction.commit().map_err(database_error)?;
        Ok(counts)
    }

    fn secret_holdings(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: Option<&str>,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>> {
        let sql = bound_on_axis(
            if holder.is_some() {
                SECRETS_OF_HOLDER
            } else {
                SECRETS
            },
            scope.axis.is_some(),
        );
        let version = to_i64(as_of_version)?;
        let scope = ScopeBinding::of(scope)?;
        let mut parameters = scope.with(&[
            (":project_id", &project_id),
            (":tick", &tick),
            (":version", &version),
        ]);
        if let Some(holder) = &holder {
            parameters.push((":holder", holder));
        }
        record_rows(&self.connection, &self.provenances, &sql, &parameters[..])
    }

    fn holdings_told_from(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        source_record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>> {
        let version = to_i64(as_of_version)?;
        let scope = ScopeBinding::of(scope)?;
        let parameters = scope.with(&[
            (":project_id", &project_id),
            (":source", &source_record_id),
            (":tick", &tick),
            (":version", &version),
        ]);
        record_rows(
            &self.connection,
            &self.provenances,
            &bound_on_axis(TOLD_FROM, scope.axis.is_some()),
            &parameters[..],
        )
    }

    fn rebase_draft(&mut self, project_id: &str, timeline_id: &str) -> Result<Timeline> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let draft = existing_draft(&transaction, project_id, timeline_id)?;
        require_active(&draft)?;
        let parent_id = parent_of(&draft)?;
        let (parent_head, parent_max_tick) = timeline_window(&transaction, project_id, parent_id)?;
        let base = draft.base_version.unwrap_or_default();
        if parent_head == base {
            return Ok(draft);
        }
        // Children read this draft's rows by version; a rebase renumbers them.
        require_no_descendants(&transaction, project_id, timeline_id, false, "rebased")?;
        let commits =
            stored_draft_commits(&transaction, &self.provenances, project_id, timeline_id)?;
        delete_draft_rows(&transaction, project_id, timeline_id)?;
        transaction
            .execute(
                "UPDATE timelines SET base_version = ?3, head_version = ?3, max_tick = ?4 \
                 WHERE project_id = ?1 AND id = ?2",
                params![
                    project_id,
                    timeline_id,
                    to_i64(parent_head)?,
                    parent_max_tick
                ],
            )
            .map_err(database_error)?;
        let caches = WriteCaches::default();
        for prepared in &commits {
            let applied = write_target(&transaction, prepared).and_then(|target| {
                // An ancestor may already contain this very attempt (same
                // request, applied there directly): then it is not applied
                // twice.
                match replay_outcome(&transaction, prepared, &target.scope)? {
                    Some(_) => Ok(()),
                    // The next commit reads its target again, so the head is
                    // stored at once.
                    None => {
                        let committed = commit_new(
                            &transaction,
                            prepared,
                            &target,
                            &caches,
                            &self.provenances,
                        )?;
                        store_head(
                            &transaction,
                            &prepared.input.project_id,
                            &target.advanced(&committed),
                        )
                    }
                }
            });
            if let Err(error) = applied {
                let cause = error.details.clone();
                return Err(error.with_details(json!({
                    "timelineId": timeline_id,
                    "failedAttemptId": prepared.input.attempt_id,
                    "failedEventId": prepared.event_id,
                    "cause": cause,
                })));
            }
        }
        transaction
            .execute(
                "INSERT INTO change_feed (project_id, timeline_id, version, type, payload) \
                 VALUES (?1, ?2, ?3, 'timeline.rebased', ?4)",
                params![
                    project_id,
                    timeline_id,
                    to_i64(parent_head)?,
                    to_json(&json!({
                        "timelineId": timeline_id,
                        "fromBaseVersion": base,
                        "toBaseVersion": parent_head,
                        "commits": commits.len(),
                    }))?
                ],
            )
            .map_err(database_error)?;
        let timeline = existing_draft(&transaction, project_id, timeline_id)?;
        transaction.commit().map_err(database_error)?;
        Ok(timeline)
    }

    fn check_commit(&mut self, prepared: &PreparedCommit) -> Result<Vec<Warning>> {
        // Validation only: the transaction is never committed.
        let transaction = self.connection.transaction().map_err(database_error)?;
        let target = write_target(&transaction, prepared)?;
        let caches = WriteCaches::default();
        let view = TransactionView {
            transaction: &transaction,
            caches: &caches,
            provenances: &self.provenances,
            target: &target,
        };
        Ok(validate_commit(&view, prepared)?.warnings)
    }

    fn replace_proposal(&mut self, proposal: &Proposal) -> Result<()> {
        update_proposal(&self.connection, proposal)
    }

    fn witnessed_by(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: &str,
        event_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<bool> {
        let version = to_i64(as_of_version)?;
        let scope = ScopeBinding::of(scope)?;
        let parameters = scope.with(&[
            (":project_id", &project_id),
            (":event_id", &event_id),
            (":holder", &holder),
            (":tick", &tick),
            (":version", &version),
        ]);
        self.connection
            .prepare_cached(&bound_on_axis(WITNESSED_BY, scope.axis.is_some()))
            .map_err(database_error)?
            .query_row(&parameters[..], |row| row.get(0))
            .map_err(database_error)
    }

    fn event(&self, project_id: &str, scope: &TimelineScope, event_id: &str) -> Result<Event> {
        event_from_connection(
            &self.connection,
            &self.provenances,
            project_id,
            scope,
            event_id,
        )?
        .ok_or_else(|| UmweltError::not_found(format!("Event '{event_id}' does not exist.")))
    }

    fn record(&self, project_id: &str, scope: &TimelineScope, record_id: &str) -> Result<Record> {
        record_from_connection(
            &self.connection,
            &self.provenances,
            project_id,
            scope,
            record_id,
        )?
        .ok_or_else(|| UmweltError::not_found(format!("Record '{record_id}' does not exist.")))
    }

    fn transition(&self, project_id: &str, transition_id: &str) -> Result<Transition> {
        transition_rows(
            &self.connection,
            &self.provenances,
            &format!(
                "SELECT {TRANSITION_COLUMNS} FROM transitions t WHERE t.project_id = ?1 AND t.id = ?2"
            ),
            params![project_id, transition_id],
        )?
        .pop()
        .ok_or_else(|| {
            UmweltError::not_found(format!("Transition '{transition_id}' does not exist."))
        })
    }

    fn record_state(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<RecordState> {
        record_state_optional(
            &self.connection,
            &self.provenances,
            project_id,
            scope,
            record_id,
            tick,
            as_of_version,
        )?
        .ok_or_else(|| {
            UmweltError::not_found(format!(
                "Record '{record_id}' is not visible at the requested coordinates."
            ))
        })
    }

    fn records_at(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: Option<&str>,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<RecordState>> {
        let fast = scope.allows_current_status()
            && is_current_window(&self.connection, project_id, tick, as_of_version)?;
        let sql = bound_on_axis(
            match (holder.is_some(), fast) {
                (true, true) => HOLDER_RECORDS_CURRENT,
                (true, false) => HOLDER_RECORDS_HISTORICAL,
                (false, true) => ALL_RECORDS_CURRENT,
                (false, false) => ALL_RECORDS_HISTORICAL,
            },
            scope.axis.is_some(),
        );
        let version = to_i64(as_of_version)?;
        let scope = ScopeBinding::of(scope)?;
        let mut parameters: Vec<(&str, &dyn ToSql)> = vec![
            (":project_id", &project_id),
            (":tick", &tick),
            (":version", &version),
        ];
        if let Some(holder) = &holder {
            parameters.push((":holder", holder));
        }
        if !fast {
            parameters.extend(scope.parameters());
            parameters.extend(scope.axis_parameter());
        }
        Ok(
            record_rows(&self.connection, &self.provenances, &sql, &parameters[..])?
                .into_iter()
                .map(|record| RecordState {
                    record,
                    transition: None,
                })
                .collect(),
        )
    }

    fn query_records(&self, query: &RecordQuery) -> Result<RecordPage> {
        let Some(keys) = KeyBlobs::from_query(query) else {
            return Ok(RecordPage {
                records: Vec::new(),
                next_cursor: None,
            });
        };
        let fast = query.ancestors.is_empty()
            && query.axis.is_none()
            && is_current_window(
                &self.connection,
                &query.project_id,
                query.tick,
                query.as_of_version,
            )?;
        let sql = build_claim_query(query, fast);
        let cursor_version = query
            .cursor
            .as_ref()
            .map(|value| to_i64(value.recorded_version))
            .transpose()?;
        let limit = i64::from(query.limit.clamp(1, 1000)) + 1;
        let as_of_version = to_i64(query.as_of_version)?;
        let scope =
            ScopeBinding::new(&query.timeline_id, &query.ancestors)?.on_axis(query.axis.as_deref());
        let parameters = claim_parameters(
            &sql,
            query,
            &keys,
            &as_of_version,
            &scope,
            &cursor_version,
            &limit,
        );
        let mut statement = self
            .connection
            .prepare_cached(&sql)
            .map_err(database_error)?;
        let rows = statement
            .query_map(&parameters[..], |row| RecordRow::read(row, 0))
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)?;
        let page_limit = usize::try_from(limit - 1).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
        })?;
        let has_more = rows.len() > page_limit;
        let mut records = rows
            .into_iter()
            .take(page_limit)
            .map(|row| row.into_record(&self.connection, &self.provenances))
            .collect::<Result<Vec<_>>>()?;
        records.truncate(page_limit);
        let next_cursor = if has_more {
            records.last().map(|record| {
                let common = record_common(record);
                RecordCursor {
                    tick: common.position.tick,
                    recorded_version: common.recorded_at,
                    record_id: common.id.clone(),
                }
            })
        } else {
            None
        };
        Ok(RecordPage {
            records,
            next_cursor,
        })
    }

    fn latest_holding(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder_id: &str,
        claim_key: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Option<HoldingSummary>> {
        let Some(key) = key_blob(claim_key) else {
            return Ok(None);
        };
        let fast = scope.allows_current_status()
            && is_current_window(&self.connection, project_id, tick, as_of_version)?;
        let version = to_i64(as_of_version)?;
        let read = |row: &rusqlite::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, f64>(3)?,
            ))
        };
        let row = if fast {
            self.connection
                .prepare_cached(LATEST_HOLDING_CURRENT)
                .map_err(database_error)?
                .query_row(params![project_id, holder_id, key, tick, version], read)
        } else {
            let scope = ScopeBinding::of(scope)?;
            let parameters = scope.with(&[
                (":project_id", &project_id),
                (":holder", &holder_id),
                (":claim_key", &key),
                (":tick", &tick),
                (":version", &version),
            ]);
            self.connection
                .prepare_cached(&bound_on_axis(
                    LATEST_HOLDING_HISTORICAL,
                    scope.axis.is_some(),
                ))
                .map_err(database_error)?
                .query_row(&parameters[..], read)
        }
        .optional()
        .map_err(database_error)?;
        row.map(|(record_id, attitude, polarity, confidence)| {
            Ok(HoldingSummary {
                record_id,
                attitude: from_json(&format!("\"{attitude}\""))?,
                polarity,
                confidence,
            })
        })
        .transpose()
    }

    fn count_withheld(&self, query: &RecordQuery) -> Result<u64> {
        if query.holder_id.is_none() {
            return Ok(0);
        }
        let Some(keys) = KeyBlobs::from_query(query) else {
            return Ok(0);
        };
        let fast = query.ancestors.is_empty()
            && query.axis.is_none()
            && is_current_window(
                &self.connection,
                &query.project_id,
                query.tick,
                query.as_of_version,
            )?;
        let sql = build_withheld_count(query, fast);
        let as_of_version = to_i64(query.as_of_version)?;
        let scope =
            ScopeBinding::new(&query.timeline_id, &query.ancestors)?.on_axis(query.axis.as_deref());
        let no_cursor = None;
        let no_limit = 0;
        let parameters = claim_parameters(
            &sql,
            query,
            &keys,
            &as_of_version,
            &scope,
            &no_cursor,
            &no_limit,
        );
        let mut statement = self
            .connection
            .prepare_cached(&sql)
            .map_err(database_error)?;
        let count: i64 = statement
            .query_row(&parameters[..], |row| row.get(0))
            .map_err(database_error)?;
        to_u64(count)
    }

    fn perspective_records(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<PerspectiveRecords> {
        let version = to_i64(as_of_version)?;
        let scope_in = scope;
        let scope = ScopeBinding::of(scope)?;
        let parameters = scope.with(&[
            (":project_id", &project_id),
            (":holder", &holder_id),
            (":tick", &tick),
            (":version", &version),
        ]);
        let active = concat!(
            "AND r.tick <= :tick AND r.recorded_version <= :version AND ",
            visible!("r"),
            " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id \
             AND t.target_id = r.id AND t.tick <= :tick AND t.recorded_version <= :version AND ",
            visible!("t"),
            ")"
        );
        let axis = scope.axis.is_some();
        let awareness = record_rows(
            &self.connection,
            &self.provenances,
            &bound_on_axis(
                &format!(
                    "SELECT {RECORD_COLUMNS} FROM records r INDEXED BY claims_holder_kind_order \
                     WHERE r.project_id = :project_id AND r.holder_id = :holder AND r.kind = 'awareness' {active} \
                     ORDER BY r.tick, r.recorded_version, r.id"
                ),
                axis,
            ),
            &parameters[..],
        )?;
        let dispositions = self.dispositions(
            project_id,
            scope_in,
            holder_id,
            None,
            None,
            tick,
            as_of_version,
        )?;
        Ok(PerspectiveRecords {
            awareness,
            dispositions,
        })
    }

    fn dispositions(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder_id: &str,
        target_id: Option<&str>,
        dimension: Option<&str>,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>> {
        let version = to_i64(as_of_version)?;
        let scope = ScopeBinding::of(scope)?;
        let parameters = scope.with(&[
            (":project_id", &project_id),
            (":holder", &holder_id),
            (":target", &target_id),
            (":dimension", &dimension),
            (":tick", &tick),
            (":version", &version),
        ]);
        record_rows(
            &self.connection,
            &self.provenances,
            &bound_on_axis(DISPOSITIONS, scope.axis.is_some()),
            &parameters[..],
        )
    }

    fn project_window(&self, project_id: &str) -> Result<(Version, i64)> {
        let (version, max_tick): (i64, i64) = self
            .connection
            .query_row(
                "SELECT version, max_tick FROM projects WHERE id = ?1",
                [project_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                UmweltError::not_found(format!("Project '{project_id}' does not exist."))
            })?;
        Ok((to_u64(version)?, max_tick))
    }

    fn entity(&self, project_id: &str, entity_id: &str) -> Result<Entity> {
        query_json_optional(
            &self.connection,
            "SELECT payload FROM entities WHERE project_id = ?1 AND id = ?2",
            params![project_id, entity_id],
        )?
        .ok_or_else(|| UmweltError::not_found(format!("Entity '{entity_id}' does not exist.")))
    }

    fn entities(&self, project_id: &str, limit: u32, cursor: Option<&str>) -> Result<EntityPage> {
        let limit = i64::from(limit.clamp(1, 1000)) + 1;
        let mut statement = self
            .connection
            .prepare_cached(
                "SELECT payload, id FROM entities WHERE project_id = ?1 AND (?2 IS NULL OR id > ?2) ORDER BY id LIMIT ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![project_id, cursor, limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let page_limit = usize::try_from(limit - 1).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
        })?;
        let mut entities = Vec::new();
        let mut ids = Vec::new();
        for row in rows {
            let (payload, id) = row.map_err(database_error)?;
            entities.push(from_json(&payload)?);
            ids.push(id);
        }
        let has_more = entities.len() > page_limit;
        if has_more {
            entities.truncate(page_limit);
            ids.truncate(page_limit);
        }
        Ok(EntityPage {
            entities,
            next_cursor: has_more.then(|| ids.last().cloned()).flatten(),
        })
    }

    fn predicates(&self, project_id: &str) -> Result<Vec<PredicateDef>> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT payload FROM predicates WHERE project_id = ?1 ORDER BY id")
            .map_err(database_error)?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        rows.map(|row| from_json(&row.map_err(database_error)?))
            .collect()
    }

    fn transitions_for_record(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        record_id: &str,
    ) -> Result<Vec<Transition>> {
        let scope = ScopeBinding::new(&scope.timeline_id, &scope.ancestors)?;
        transition_rows(
            &self.connection,
            &self.provenances,
            concat!(
                "SELECT ",
                transition_columns!(),
                " FROM transitions t WHERE t.project_id = :project_id AND t.target_id = :id AND ",
                visible!("t"),
                " ORDER BY t.tick, t.recorded_version, t.ordinal, t.id"
            ),
            &scope.with(&[(":project_id", &project_id), (":id", &record_id)])[..],
        )
    }

    fn events(&self, query: &EventQuery) -> Result<EventPage> {
        let limit = i64::from(query.limit.clamp(1, 1000)) + 1;
        let cursor_tick = query.cursor.as_ref().map(|cursor| cursor.tick);
        let cursor_version = query
            .cursor
            .as_ref()
            .map(|cursor| to_i64(cursor.recorded_version))
            .transpose()?;
        let cursor_id = query.cursor.as_ref().map(|cursor| cursor.event_id.as_str());
        let as_of_version = to_i64(query.as_of_version)?;
        let rows = if let Some(axis) = &query.axis {
            let scope = ScopeBinding::new(&query.timeline_id, &query.ancestors)?;
            let parameters = scope.with(&[
                (":project_id", &query.project_id),
                (":axis", axis),
                (":tick", &query.tick),
                (":version", &as_of_version),
                (":cursor_tick", &cursor_tick),
                (":cursor_version", &cursor_version),
                (":cursor_id", &cursor_id),
                (":limit", &limit),
            ]);
            self.connection
                .prepare_cached(EVENTS_ON_AXIS)
                .map_err(database_error)?
                .query_map(&parameters[..], |row| EventRow::read(row, 0))
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
        } else if query.ancestors.is_empty() {
            let parameters: [&dyn ToSql; 8] = [
                &query.project_id,
                &query.timeline_id,
                &query.tick,
                &as_of_version,
                &cursor_tick,
                &cursor_version,
                &cursor_id,
                &limit,
            ];
            self.connection
                .prepare_cached(EVENTS_MAIN)
                .map_err(database_error)?
                .query_map(&parameters[..], |row| EventRow::read(row, 0))
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
        } else {
            let scope = ScopeBinding::new(&query.timeline_id, &query.ancestors)?;
            let parameters = scope.with(&[
                (":project_id", &query.project_id),
                (":tick", &query.tick),
                (":version", &as_of_version),
                (":cursor_tick", &cursor_tick),
                (":cursor_version", &cursor_version),
                (":cursor_id", &cursor_id),
                (":limit", &limit),
            ]);
            self.connection
                .prepare_cached(EVENTS_DRAFT)
                .map_err(database_error)?
                .query_map(&parameters[..], |row| EventRow::read(row, 0))
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
        }
        .map_err(database_error)?;
        let page_limit = usize::try_from(limit - 1).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
        })?;
        let has_more = rows.len() > page_limit;
        let events = rows
            .into_iter()
            .take(page_limit)
            .map(|row| row.into_event(&self.connection, &self.provenances))
            .collect::<Result<Vec<Event>>>()?;
        let next_cursor = if has_more {
            events.last().map(|event| EventCursor {
                // On an axis the page is in axis order, and every event read
                // has a position there.
                tick: match &query.axis {
                    Some(axis) => event.on.get(axis).copied().unwrap_or_default(),
                    None => event.position.tick,
                },
                recorded_version: event.recorded_at,
                event_id: event.id.clone(),
            })
        } else {
            None
        };
        Ok(EventPage {
            events,
            next_cursor,
        })
    }

    fn changes(
        &self,
        project_id: &str,
        since_seq: u64,
        limit: u32,
        timeline_id: Option<&str>,
    ) -> Result<ChangePage> {
        let limit = i64::from(limit.clamp(1, 1000)) + 1;
        let mut statement = self.connection.prepare_cached(
            "SELECT seq, version, type, payload, timeline_id, event_id, record_count, transition_count \
             FROM change_feed WHERE project_id = ?1 AND seq > ?2 AND (?4 IS NULL OR timeline_id = ?4) \
             ORDER BY seq LIMIT ?3",
        ).map_err(database_error)?;
        let rows = statement
            .query_map(
                params![project_id, to_i64(since_seq)?, limit, timeline_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                        row.get::<_, Option<i64>>(7)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let page_limit = usize::try_from(limit - 1).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
        })?;
        let mut changes = Vec::new();
        for row in rows {
            let (seq, version, change_type, payload, timeline_id, event_id, records, transitions) =
                row.map_err(database_error)?;
            let version = to_u64(version)?;
            // Commit items are stored as columns and rebuilt here.
            let payload = match payload {
                Some(payload) => from_json(&payload)?,
                None => json!({
                    "eventId": event_id,
                    "version": version,
                    "recordCount": records,
                    "transitionCount": transitions,
                }),
            };
            changes.push(ChangeItem {
                seq: to_u64(seq)?,
                timeline_id,
                version,
                change_type,
                payload,
            });
        }
        let has_more = changes.len() > page_limit;
        if has_more {
            changes.truncate(page_limit);
        }
        let next_seq = has_more
            .then(|| changes.last().map(|item| item.seq))
            .flatten();
        Ok(ChangePage { changes, next_seq })
    }

    fn search(&self, query: &SearchQuery) -> Result<SearchPage> {
        search_records(&self.connection, &self.provenances, query)
    }

    fn insert_proposal(&mut self, proposal: &Proposal) -> Result<()> {
        self.connection
            .execute(
                "INSERT INTO proposals (project_id, id, status, rationale, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    proposal.input.project_id,
                    proposal.id,
                    status_name(proposal.status),
                    proposal.rejection_rationale,
                    to_json(proposal)?
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn proposal(&self, proposal_id: &str) -> Result<Proposal> {
        proposal_from_connection(&self.connection, proposal_id)?.ok_or_else(|| {
            UmweltError::not_found(format!("Proposal '{proposal_id}' does not exist."))
        })
    }

    fn proposals(&self, project_id: &str) -> Result<Vec<Proposal>> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT payload FROM proposals WHERE project_id = ?1 ORDER BY id")
            .map_err(database_error)?;
        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        rows.map(|row| from_json(&row.map_err(database_error)?))
            .collect()
    }

    fn append_reviews(
        &mut self,
        proposal_id: &str,
        reviews: &[ProposalReview],
    ) -> Result<Proposal> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let mut proposal =
            proposal_from_connection(&transaction, proposal_id)?.ok_or_else(|| {
                UmweltError::not_found(format!("Proposal '{proposal_id}' does not exist."))
            })?;
        for review in reviews {
            transaction
                .execute(
                    "INSERT INTO proposal_reviews (project_id, proposal_id, operation_index, seq, payload) VALUES (?1, ?2, ?3, COALESCE((SELECT MAX(seq) + 1 FROM proposal_reviews WHERE project_id = ?1 AND proposal_id = ?2 AND operation_index = ?3), 1), ?4)",
                    params![proposal.input.project_id, proposal_id, review.operation_index, to_json(review)?],
                )
                .map_err(database_error)?;
            proposal.reviews.push(review.clone());
        }
        update_proposal(&transaction, &proposal)?;
        transaction.commit().map_err(database_error)?;
        Ok(proposal)
    }

    fn set_proposal_status(
        &mut self,
        proposal_id: &str,
        status: ProposalStatus,
        rationale: Option<&str>,
    ) -> Result<Proposal> {
        let mut proposal = self.proposal(proposal_id)?;
        proposal.status = status;
        proposal.rejection_rationale = rationale.map(str::to_owned);
        update_proposal(&self.connection, &proposal)?;
        Ok(proposal)
    }

    fn counts(&self, project_id: &str) -> Result<StoreCounts> {
        Ok(StoreCounts {
            events: count(&self.connection, "events", project_id)?,
            records: count(&self.connection, "records", project_id)?,
            transitions: count(&self.connection, "transitions", project_id)?,
            attempts: count(&self.connection, "attempts", project_id)?,
            proposals: count(&self.connection, "proposals", project_id)?,
        })
    }

    fn checkpoint(&mut self) -> Result<()> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(database_error)
    }
}

/// Entity kinds looked up during one write transaction. Entities are never
/// created or retyped inside a commit, so a kind read once stays valid for the
/// whole transaction; a transmit asks for the same two agents five times.
type EntityKindCache = RefCell<HashMap<(String, String), Option<EntityKind>>>;

/// Caches that live for one write transaction. Nothing a commit writes can
/// change what they hold: entities, predicates and a project's vocabulary are
/// never written by a commit.
#[derive(Default)]
struct WriteCaches {
    kinds: EntityKindCache,
    provenances: ProvenanceWrites,
    predicates: RefCell<HashMap<(String, String), Option<PredicateDef>>>,
    /// Projects as first read in the transaction. Their version moves with
    /// every commit; validation reads versions from the write target instead.
    projects: RefCell<HashMap<String, Option<Project>>>,
    /// Ticks of records read during validation, so checking a record that a
    /// precondition already read costs no second query.
    record_ticks: RefCell<HashMap<(String, String), i64>>,
}

/// What a commit's validation may see: rows visible in the timeline it
/// writes to, at that timeline's head.
struct TransactionView<'a> {
    transaction: &'a Transaction<'a>,
    caches: &'a WriteCaches,
    provenances: &'a ProvenanceCache,
    target: &'a WriteTarget,
}

impl ValidationView for TransactionView<'_> {
    fn project(&self, project_id: &str) -> Result<Option<Project>> {
        if let Some(project) = self.caches.projects.borrow().get(project_id) {
            return Ok(project.clone());
        }
        let project = project_from_connection(self.transaction, project_id)?;
        self.caches
            .projects
            .borrow_mut()
            .insert(project_id.to_owned(), project.clone());
        Ok(project)
    }

    fn entity_kind(&self, project_id: &str, entity_id: &str) -> Result<Option<EntityKind>> {
        let key = (project_id.to_owned(), entity_id.to_owned());
        if let Some(kind) = self.caches.kinds.borrow().get(&key) {
            return Ok(kind.clone());
        }
        let kind = self
            .transaction
            .prepare_cached(
                "SELECT et.kind FROM entities e JOIN entity_types et ON et.project_id = e.project_id AND et.id = e.type_id WHERE e.project_id = ?1 AND e.id = ?2",
            )
            .map_err(database_error)?
            .query_row(params![project_id, entity_id], |row| row.get::<_, String>(0))
            .optional()
            .map_err(database_error)?
            .map(|kind| from_json::<EntityKind>(&format!("\"{kind}\"")))
            .transpose()?;
        self.caches.kinds.borrow_mut().insert(key, kind.clone());
        Ok(kind)
    }

    fn predicate(&self, project_id: &str, predicate_id: &str) -> Result<Option<PredicateDef>> {
        let key = (project_id.to_owned(), predicate_id.to_owned());
        if let Some(predicate) = self.caches.predicates.borrow().get(&key) {
            return Ok(predicate.clone());
        }
        let predicate: Option<PredicateDef> = query_json_optional(
            self.transaction,
            "SELECT payload FROM predicates WHERE project_id = ?1 AND id = ?2",
            params![project_id, predicate_id],
        )?;
        self.caches
            .predicates
            .borrow_mut()
            .insert(key, predicate.clone());
        Ok(predicate)
    }

    fn record(&self, project_id: &str, record_id: &str) -> Result<Option<Record>> {
        record_from_connection(
            self.transaction,
            self.provenances,
            project_id,
            &self.target.scope,
            record_id,
        )
    }

    fn record_state(
        &self,
        project_id: &str,
        record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Option<RecordState>> {
        let state = record_state_optional(
            self.transaction,
            self.provenances,
            project_id,
            &self.target.scope,
            record_id,
            tick,
            as_of_version,
        )?;
        if let Some(state) = &state {
            // Records are immutable and stay visible to this timeline, so the
            // tick holds for the rest of the transaction.
            self.caches.record_ticks.borrow_mut().insert(
                (project_id.to_owned(), record_id.to_owned()),
                record_common(&state.record).position.tick,
            );
        }
        Ok(state)
    }

    fn event_exists(&self, project_id: &str, event_id: &str) -> Result<bool> {
        let scope = ScopeBinding::of(&self.target.scope)?;
        Ok(self
            .transaction
            .prepare_cached(concat!(
                "SELECT 1 FROM events e WHERE e.project_id = :project_id AND e.id = :id AND ",
                visible!("e")
            ))
            .map_err(database_error)?
            .query_row(
                &scope.with(&[(":project_id", &project_id), (":id", &event_id)])[..],
                |_| Ok(()),
            )
            .optional()
            .map_err(database_error)?
            .is_some())
    }

    fn record_exists(&self, project_id: &str, record_id: &str) -> Result<bool> {
        Ok(self.record_tick(project_id, record_id)?.is_some())
    }

    fn record_tick(&self, project_id: &str, record_id: &str) -> Result<Option<i64>> {
        let key = (project_id.to_owned(), record_id.to_owned());
        if let Some(tick) = self.caches.record_ticks.borrow().get(&key) {
            return Ok(Some(*tick));
        }
        let scope = ScopeBinding::of(&self.target.scope)?;
        self.transaction
            .prepare_cached(concat!(
                "SELECT r.tick FROM records r WHERE r.project_id = :project_id AND r.id = :id AND ",
                visible!("r")
            ))
            .map_err(database_error)?
            .query_row(
                &scope.with(&[(":project_id", &project_id), (":id", &record_id)])[..],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)
    }

    fn head_version(&self, _project: &Project) -> Result<Version> {
        Ok(self.target.head_version)
    }

    fn secret_shared_with(&self, project_id: &str, record_id: &str) -> Result<Option<Vec<String>>> {
        let shared: Option<Option<String>> = self
            .transaction
            .prepare_cached("SELECT shared_with FROM records WHERE project_id = ?1 AND id = ?2")
            .map_err(database_error)?
            .query_row(params![project_id, record_id], |row| row.get(0))
            .optional()
            .map_err(database_error)?;
        shared.flatten().map(|text| from_json(&text)).transpose()
    }

    fn axis_exists(&self, project_id: &str, axis_id: &str) -> Result<bool> {
        self.transaction
            .prepare_cached("SELECT EXISTS(SELECT 1 FROM axes WHERE project_id = ?1 AND id = ?2)")
            .map_err(database_error)?
            .query_row(params![project_id, axis_id], |row| row.get(0))
            .map_err(database_error)
    }

    fn operation_id_exists(&self, project_id: &str, id: &str) -> Result<bool> {
        // One statement instead of three point lookups per id.
        self.transaction
            .prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM events WHERE project_id = ?1 AND id = ?2)                  OR EXISTS(SELECT 1 FROM records WHERE project_id = ?1 AND id = ?2)                  OR EXISTS(SELECT 1 FROM transitions WHERE project_id = ?1 AND id = ?2)",
            )
            .map_err(database_error)?
            .query_row(params![project_id, id], |row| row.get(0))
            .map_err(database_error)
    }
}

fn insert_event(transaction: &Transaction<'_>, caches: &WriteCaches, event: &Event) -> Result<()> {
    let provenance_id =
        caches
            .provenances
            .intern(transaction, &event.project_id, &event.provenance)?;
    insert_event_row(transaction, event, provenance_id)?;
    // Events the engine generates itself (type "umwelt:*", e.g. one per
    // transmit) carry boilerplate titles; indexing them only bloats search.
    match event_search_text(event) {
        Some(text) => insert_search(transaction, &event.project_id, EVENT_DOC, &event.id, &text),
        None => Ok(()),
    }
}

fn insert_record(
    transaction: &Transaction<'_>,
    caches: &WriteCaches,
    record: &Record,
) -> Result<()> {
    let common = record_common(record);
    let provenance_id =
        caches
            .provenances
            .intern(transaction, &common.project_id, &common.provenance)?;
    RecordColumns::from_record(record, provenance_id)?.insert(transaction)?;
    // The same text `embedding.pending` hands to the client's model.
    match record_search_text(record) {
        Some(text) => insert_search(
            transaction,
            &common.project_id,
            RECORD_DOC,
            &common.id,
            &text,
        ),
        None => Ok(()),
    }
}

/// Executes through the statement cache. A commit batch repeats the same
/// handful of INSERTs thousands of times; recompiling them dominated writes.
fn execute_cached<P: rusqlite::Params>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> Result<()> {
    connection
        .prepare_cached(sql)
        .map_err(database_error)?
        .execute(params)
        .map_err(database_error)?;
    Ok(())
}

fn insert_search(
    connection: &Connection,
    project_id: &str,
    doc_type: &str,
    doc_id: &str,
    text: &str,
) -> Result<()> {
    if text.trim().is_empty() {
        return Ok(());
    }
    execute_cached(
        connection,
        "INSERT INTO search_fts (project_id, doc_type, doc_id, text) VALUES (?1, ?2, ?3, ?4)",
        params![project_id, doc_type, doc_id, text],
    )
}

fn insert_transition(
    transaction: &Transaction<'_>,
    caches: &WriteCaches,
    transition: &Transition,
) -> Result<()> {
    let provenance_id =
        caches
            .provenances
            .intern(transaction, &transition.project_id, &transition.provenance)?;
    insert_transition_row(transaction, transition, provenance_id)?;
    execute_cached(
        transaction,
        "INSERT INTO current_status (project_id, timeline_id, record_id, status, changed_by, changed_tick, changed_version) VALUES (?1, ?7, ?2, ?3, ?4, ?5, ?6) ON CONFLICT(project_id, timeline_id, record_id) DO UPDATE SET status = excluded.status, changed_by = excluded.changed_by, changed_tick = excluded.changed_tick, changed_version = excluded.changed_version WHERE excluded.changed_tick > current_status.changed_tick OR (excluded.changed_tick = current_status.changed_tick AND excluded.changed_version >= current_status.changed_version)",
        params![
            transition.project_id,
            transition.target_record_id,
            enum_name(&transition.status)?,
            transition.id,
            transition.position.tick,
            to_i64(transition.recorded_at)?,
            transition.timeline_id
        ],
    )
}

fn insert_project(connection: &Connection, project: &Project) -> Result<()> {
    connection
        .execute(
            "INSERT INTO projects (id, name, description, timescale, vocabulary_mode, created_at, version, last_version) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![project.id, project.name, project.description, project.timescale, enum_name(&project.vocabulary_mode)?, project.created_at, to_i64(project.version)?],
        )
        .map_err(|source| {
            duplicate_or_database_error(source, |_| {
                format!("Project '{}' already exists.", project.id)
            })
        })?;
    for predicate in reserved_predicates() {
        insert_predicate(connection, &project.id, &predicate)?;
    }
    Ok(())
}

fn insert_entity_type(connection: &Connection, entity_type: &EntityType) -> Result<()> {
    connection
        .execute(
            "INSERT INTO entity_types (project_id, id, name, kind, payload) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entity_type.project_id,
                entity_type.id,
                entity_type.name,
                enum_name(&entity_type.kind)?,
                to_json(entity_type)?
            ],
        )
        .map_err(|source| {
            duplicate_or_database_error(source, |_| {
                format!(
                    "Entity type '{}' already exists in project '{}'.",
                    entity_type.id, entity_type.project_id
                )
            })
        })?;
    Ok(())
}

fn insert_predicate(
    connection: &Connection,
    project_id: &str,
    predicate: &PredicateDef,
) -> Result<()> {
    connection
        .execute(
            "INSERT INTO predicates (project_id, id, payload) VALUES (?1, ?2, ?3)",
            params![project_id, predicate.id, to_json(predicate)?],
        )
        .map_err(|source| {
            duplicate_or_database_error(source, |_| {
                format!(
                    "Predicate '{}' already exists in project '{project_id}'.",
                    predicate.id
                )
            })
        })?;
    Ok(())
}

fn insert_entity(connection: &Connection, entity: &Entity) -> Result<()> {
    project_from_connection(connection, &entity.project_id)?.ok_or_else(|| {
        UmweltError::not_found(format!("Project '{}' does not exist.", entity.project_id))
    })?;
    let entity_type: EntityType = connection
        .query_row(
            "SELECT payload FROM entity_types WHERE project_id = ?1 AND id = ?2",
            params![entity.project_id, entity.type_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            UmweltError::not_found(format!("Entity type '{}' does not exist.", entity.type_id))
        })
        .and_then(|data: String| from_json(&data))?;
    validate_entity_fields(&entity.fields, &entity_type)?;
    connection
        .execute(
            "INSERT INTO entities (project_id, id, type_id, name, external_system, external_id, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![entity.project_id, entity.id, entity.type_id, entity.name, entity.external_ref.as_ref().map(|value| value.system.as_str()), entity.external_ref.as_ref().map(|value| value.id.as_str()), to_json(entity)?],
        )
        .map_err(|source| {
            duplicate_or_database_error(source, |duplicate| match (duplicate, &entity.external_ref) {
                (Duplicate::Unique, Some(reference)) => format!(
                    "Another entity in project '{}' already has the external reference '{}:{}'.",
                    entity.project_id, reference.system, reference.id
                ),
                _ => format!(
                    "Entity '{}' already exists in project '{}'.",
                    entity.id, entity.project_id
                ),
            })
        })?;
    let aliases = entity.aliases.join(" ");
    insert_search(
        connection,
        &entity.project_id,
        "entity",
        &entity.id,
        &format!("{} {aliases}", entity.name),
    )?;
    Ok(())
}

fn replace_entity(connection: &Connection, entity: &Entity) -> Result<()> {
    let entity_type: EntityType = query_json_optional(
        connection,
        "SELECT payload FROM entity_types WHERE project_id = ?1 AND id = ?2",
        params![entity.project_id, entity.type_id],
    )?
    .ok_or_else(|| {
        UmweltError::not_found(format!("Entity type '{}' does not exist.", entity.type_id))
    })?;
    validate_entity_fields(&entity.fields, &entity_type)?;
    let changed = connection.execute(
        "UPDATE entities SET type_id = ?3, name = ?4, external_system = ?5, external_id = ?6, payload = ?7 WHERE project_id = ?1 AND id = ?2",
        params![entity.project_id, entity.id, entity.type_id, entity.name, entity.external_ref.as_ref().map(|value| value.system.as_str()), entity.external_ref.as_ref().map(|value| value.id.as_str()), to_json(entity)?],
    ).map_err(database_error)?;
    if changed == 0 {
        return Err(UmweltError::not_found(format!(
            "Entity '{}' does not exist.",
            entity.id
        )));
    }
    connection
        .execute(
            "DELETE FROM search_fts WHERE project_id = ?1 AND doc_type = 'entity' AND doc_id = ?2",
            params![entity.project_id, entity.id],
        )
        .map_err(database_error)?;
    insert_search(
        connection,
        &entity.project_id,
        "entity",
        &entity.id,
        &format!("{} {}", entity.name, entity.aliases.join(" ")),
    )?;
    Ok(())
}

fn latest_transition(
    connection: &Connection,
    provenances: &ProvenanceCache,
    project_id: &str,
    scope: &TimelineScope,
    record_id: &str,
    tick: i64,
    version: Version,
) -> Result<Option<Transition>> {
    let version = to_i64(version)?;
    let scope = ScopeBinding::of(scope)?;
    Ok(transition_rows(
        connection,
        provenances,
        &bound_on_axis(LATEST_TRANSITION, scope.axis.is_some()),
        &scope.with(&[
            (":project_id", &project_id),
            (":id", &record_id),
            (":tick", &tick),
            (":version", &version),
        ])[..],
    )?
    .pop())
}

fn record_state_optional(
    connection: &Connection,
    provenances: &ProvenanceCache,
    project_id: &str,
    scope: &TimelineScope,
    record_id: &str,
    tick: i64,
    as_of_version: Version,
) -> Result<Option<RecordState>> {
    let version = to_i64(as_of_version)?;
    let binding = ScopeBinding::of(scope)?;
    let record = record_rows(
        connection,
        provenances,
        &bound_on_axis(RECORD_AT, scope.axis.is_some()),
        &binding.with(&[
            (":project_id", &project_id),
            (":id", &record_id),
            (":tick", &tick),
            (":version", &version),
        ])[..],
    )?
    .pop();
    let Some(record) = record else {
        return Ok(None);
    };
    let transition = latest_transition(
        connection,
        provenances,
        project_id,
        scope,
        record_id,
        tick,
        as_of_version,
    )?;
    Ok(Some(RecordState { record, transition }))
}

/// The timeline a commit writes to, read inside the write transaction.
struct WriteTarget {
    scope: TimelineScope,
    head_version: Version,
    last_version: Version,
    /// Highest tick written since the target was read; `None` before the
    /// first commit.
    max_tick: Option<i64>,
}

/// A commit applied in the transaction; its head is not stored yet.
struct Committed {
    outcome: CommitOutcome,
    maximum_tick: i64,
}

impl WriteTarget {
    /// The target after `committed`, kept in memory until `store_head`.
    fn advanced(self, committed: &Committed) -> Self {
        Self {
            head_version: committed.outcome.version,
            last_version: committed.outcome.version,
            max_tick: Some(self.max_tick.map_or(committed.maximum_tick, |tick| {
                tick.max(committed.maximum_tick)
            })),
            ..self
        }
    }
}

/// Writes the versions and highest tick that commits moved in memory: main's
/// window on the project row, a draft's on its timeline row. A lot of commits
/// write this once, not once per commit.
fn store_head(connection: &Connection, project_id: &str, target: &WriteTarget) -> Result<()> {
    let Some(max_tick) = target.max_tick else {
        return Ok(());
    };
    if target.scope.is_main() {
        execute_cached(
            connection,
            "UPDATE projects SET version = ?2, last_version = ?2, max_tick = MAX(max_tick, ?3) WHERE id = ?1",
            params![project_id, to_i64(target.head_version)?, max_tick],
        )
    } else {
        execute_cached(
            connection,
            "UPDATE projects SET last_version = ?2 WHERE id = ?1",
            params![project_id, to_i64(target.last_version)?],
        )?;
        execute_cached(
            connection,
            "UPDATE timelines SET head_version = ?3, max_tick = MAX(max_tick, ?4) WHERE project_id = ?1 AND id = ?2",
            params![
                project_id,
                target.scope.timeline_id,
                to_i64(target.head_version)?,
                max_tick
            ],
        )
    }
}

fn write_target(connection: &Connection, prepared: &PreparedCommit) -> Result<WriteTarget> {
    let project_id = &prepared.input.project_id;
    let timeline_id = prepared.input.timeline();
    if timeline_id == MAIN_TIMELINE_ID {
        let (head, last): (i64, i64) = connection
            .prepare_cached("SELECT version, last_version FROM projects WHERE id = ?1")
            .map_err(database_error)?
            .query_row([project_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                UmweltError::not_found(format!("Project '{project_id}' does not exist."))
            })?;
        return Ok(WriteTarget {
            scope: TimelineScope::main(),
            head_version: to_u64(head)?,
            last_version: to_u64(last)?,
            max_tick: None,
        });
    }
    let row: Option<(String, String, Option<i64>, i64, i64)> = connection
        .prepare_cached(
            "SELECT t.status, t.parent_id, t.base_version, t.head_version, p.last_version \
             FROM timelines t JOIN projects p ON p.id = t.project_id \
             WHERE t.project_id = ?1 AND t.id = ?2",
        )
        .map_err(database_error)?
        .query_row(params![project_id, timeline_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .optional()
        .map_err(database_error)?;
    let Some((status, parent, base, head, last)) = row else {
        return Err(UmweltError::not_found(format!(
            "Timeline '{timeline_id}' does not exist."
        )));
    };
    if status != "active" {
        return Err(UmweltError::public(
            UmweltErrorCode::InvalidState,
            format!("Timeline '{timeline_id}' is {status} and no longer accepts writes."),
        )
        .with_details(json!({ "timelineId": timeline_id, "status": status })));
    }
    let ancestors = ancestry(
        connection,
        project_id,
        &parent,
        to_u64(base.unwrap_or_default())?,
    )?;
    Ok(WriteTarget {
        scope: TimelineScope::nested(timeline_id, ancestors),
        head_version: to_u64(head)?,
        last_version: to_u64(last)?,
        max_tick: None,
    })
}

/// An attempt is looked up where the commit could see it: in main for main;
/// in the draft and in its ancestors (as `visible!` reads them) for a draft.
fn replay_outcome(
    connection: &Connection,
    prepared: &PreparedCommit,
    scope: &TimelineScope,
) -> Result<Option<CommitOutcome>> {
    let Some(stored) = stored_attempt(
        connection,
        &prepared.input.project_id,
        &prepared.input.attempt_id,
        scope,
    )?
    else {
        return Ok(None);
    };
    if stored.fingerprint != prepared.fingerprint {
        return Err(UmweltError::public(
            UmweltErrorCode::Conflict,
            "attemptId was already used with a different payload.",
        )
        .with_details(json!({ "attemptId": prepared.input.attempt_id })));
    }
    Ok(Some(stored.outcome))
}

/// An attempt as stored: its request fingerprint, outcome and preconditions.
struct StoredAttempt {
    fingerprint: String,
    outcome: CommitOutcome,
    preconditions: Vec<Precondition>,
}

fn stored_attempt(
    connection: &Connection,
    project_id: &str,
    attempt_id: &str,
    scope: &TimelineScope,
) -> Result<Option<StoredAttempt>> {
    let read = |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    let row: Option<(Vec<u8>, String, i64, Option<String>)> = if scope.is_main() {
        connection
            .prepare_cached(
                "SELECT fingerprint, event_id, recorded_version, extra FROM attempts \
                 WHERE project_id = ?1 AND timeline_id = 'main' AND attempt_id = ?2",
            )
            .map_err(database_error)?
            .query_row(params![project_id, attempt_id], read)
    } else {
        // The IN list lets each timeline of the scope be a primary-key lookup;
        // the draft's own attempt wins over an ancestor's.
        let scope = ScopeBinding::of(scope)?;
        connection
            .prepare_cached(concat!(
                "SELECT a.fingerprint, a.event_id, a.recorded_version, a.extra FROM attempts a \
                 WHERE a.project_id = :project_id \
                 AND a.timeline_id IN (:timeline_id, :a1, :a2, :a3, :a4) \
                 AND a.attempt_id = :attempt_id AND ",
                visible!("a"),
                " ORDER BY a.timeline_id = :timeline_id DESC, a.recorded_version DESC LIMIT 1"
            ))
            .map_err(database_error)?
            .query_row(
                &scope.with(&[(":project_id", &project_id), (":attempt_id", &attempt_id)])[..],
                read,
            )
    }
    .optional()
    .map_err(database_error)?;
    row.map(|(fingerprint, event_id, version, extra)| {
        stored_attempt_from(fingerprint, event_id, version, extra)
    })
    .transpose()
}

fn stored_attempt_from(
    fingerprint: Vec<u8>,
    event_id: String,
    version: i64,
    extra: Option<String>,
) -> Result<StoredAttempt> {
    let extra = AttemptExtra::decode(extra.as_deref())?;
    Ok(StoredAttempt {
        fingerprint: fingerprint_hex(&fingerprint),
        outcome: CommitOutcome {
            event_id,
            record_ids: extra.record_ids,
            transition_ids: extra.transition_ids,
            version: to_u64(version)?,
            warnings: extra.warnings,
        },
        preconditions: extra.preconditions,
    })
}

fn commit_new(
    transaction: &Transaction<'_>,
    prepared: &PreparedCommit,
    target: &WriteTarget,
    caches: &WriteCaches,
    provenances: &ProvenanceCache,
) -> Result<Committed> {
    let current_version = target.head_version;
    if let Some(expected) = prepared.input.expected_version {
        if expected != current_version {
            return Err(UmweltError::public(
                UmweltErrorCode::VersionConflict,
                format!(
                    "Expected timeline version {expected}, but current version is {current_version}."
                ),
            )
            .with_details(json!({
                "timelineId": target.scope.timeline_id,
                "expectedVersion": expected,
                "headVersion": current_version,
            })));
        }
    }
    let validated = {
        let view = TransactionView {
            transaction,
            caches,
            provenances,
            target,
        };
        validate_commit(&view, prepared)?
    };
    // Versions come from one counter per project, so they never repeat
    // across timelines.
    let version = target.last_version.checked_add(1).ok_or_else(|| {
        UmweltError::public(UmweltErrorCode::InvalidState, "Project version overflowed.")
    })?;
    let (event, records, transitions, outcome) = materialize_commit(prepared, version, validated);
    let maximum_tick = records
        .iter()
        .map(|record| record_common(record).position.tick)
        .chain(
            transitions
                .iter()
                .map(|transition| transition.position.tick),
        )
        .fold(event.position.tick, i64::max);
    insert_event(transaction, caches, &event)?;
    for record in &records {
        insert_record(transaction, caches, record)?;
    }
    for transition in &transitions {
        insert_transition(transaction, caches, transition)?;
    }
    let extra = AttemptExtra {
        record_ids: outcome.record_ids.clone(),
        transition_ids: outcome.transition_ids.clone(),
        warnings: outcome.warnings.clone(),
        // Only a draft is ever rebased, so only draft attempts keep them.
        preconditions: if target.scope.is_main() {
            Vec::new()
        } else {
            prepared.input.preconditions.clone()
        },
    };
    execute_cached(
        transaction,
        "INSERT INTO attempts (project_id, timeline_id, attempt_id, fingerprint, event_id, \
         recorded_version, extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            prepared.input.project_id,
            target.scope.timeline_id,
            prepared.input.attempt_id,
            fingerprint_blob(&prepared.fingerprint)?,
            outcome.event_id,
            to_i64(version)?,
            extra.encode()?
        ],
    )?;
    // The feed stores what changed as columns; `changes` rebuilds the payload.
    execute_cached(
        transaction,
        "INSERT INTO change_feed (project_id, timeline_id, version, type, event_id, \
         record_count, transition_count) VALUES (?1, ?2, ?3, 'commit', ?4, ?5, ?6)",
        params![
            prepared.input.project_id,
            target.scope.timeline_id,
            to_i64(version)?,
            outcome.event_id,
            outcome.record_ids.len(),
            outcome.transition_ids.len()
        ],
    )?;
    Ok(Committed {
        outcome,
        maximum_tick,
    })
}

fn project_from_connection(connection: &Connection, project_id: &str) -> Result<Option<Project>> {
    connection.prepare_cached(
        "SELECT id, name, description, timescale, vocabulary_mode, created_at, version FROM projects WHERE id = ?1",
    ).map_err(database_error)?.query_row(
        [project_id],
        |row| {
            let vocabulary: String = row.get(4)?;
            let version: i64 = row.get(6)?;
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, String>(3)?, vocabulary, row.get::<_, String>(5)?, version))
        },
    ).optional().map_err(database_error)?.map(|(id, name, description, timescale, vocabulary, created_at, version)| {
        Ok(Project { id, name, description, timescale, vocabulary_mode: from_json(&format!("\"{vocabulary}\""))?, created_at, version: to_u64(version)? })
    }).transpose()
}

fn record_from_connection(
    connection: &Connection,
    provenances: &ProvenanceCache,
    project_id: &str,
    scope: &TimelineScope,
    record_id: &str,
) -> Result<Option<Record>> {
    let scope = ScopeBinding::new(&scope.timeline_id, &scope.ancestors)?;
    Ok(record_rows(
        connection,
        provenances,
        RECORD_BY_ID,
        &scope.with(&[(":project_id", &project_id), (":id", &record_id)])[..],
    )?
    .pop())
}

fn event_from_connection(
    connection: &Connection,
    provenances: &ProvenanceCache,
    project_id: &str,
    scope: &TimelineScope,
    event_id: &str,
) -> Result<Option<Event>> {
    let scope = ScopeBinding::new(&scope.timeline_id, &scope.ancestors)?;
    let row = connection
        .prepare_cached(EVENT_BY_ID)
        .map_err(database_error)?
        .query_row(
            &scope.with(&[(":project_id", &project_id), (":id", &event_id)])[..],
            |row| EventRow::read(row, 0),
        )
        .optional()
        .map_err(database_error)?;
    row.map(|row| row.into_event(connection, provenances))
        .transpose()
}

/// Runs a query whose first columns are `RECORD_COLUMNS` and rebuilds each
/// record.
fn record_rows<P: rusqlite::Params>(
    connection: &Connection,
    provenances: &ProvenanceCache,
    sql: &str,
    parameters: P,
) -> Result<Vec<Record>> {
    let rows = connection
        .prepare_cached(sql)
        .map_err(database_error)?
        .query_map(parameters, |row| RecordRow::read(row, 0))
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    rows.into_iter()
        .map(|row| row.into_record(connection, provenances))
        .collect()
}

fn transition_rows<P: rusqlite::Params>(
    connection: &Connection,
    provenances: &ProvenanceCache,
    sql: &str,
    parameters: P,
) -> Result<Vec<Transition>> {
    let rows = connection
        .prepare_cached(sql)
        .map_err(database_error)?
        .query_map(parameters, TransitionRow::read)
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    rows.into_iter()
        .map(|row| row.into_transition(connection, provenances))
        .collect()
}

/// One `visible!` slot per ancestor a draft can have.
const ANCESTOR_SLOTS: usize = MAX_DRAFT_DEPTH as usize;

/// Bind values of a `TimelineScope` for `visible!`: the timeline read and one
/// slot per possible ancestor, NULL when unused (always, for main).
struct ScopeBinding {
    timeline_id: String,
    ids: [Option<String>; ANCESTOR_SLOTS],
    versions: [Option<i64>; ANCESTOR_SLOTS],
    /// `:axis` of a positioned read on an additional axis (`on_axis!`).
    axis: Option<String>,
}

impl ScopeBinding {
    fn new(timeline_id: &str, ancestors: &[ScopeAncestor]) -> Result<Self> {
        if ancestors.len() > ANCESTOR_SLOTS {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                format!("Timeline '{timeline_id}' is nested deeper than {MAX_DRAFT_DEPTH} drafts."),
            ));
        }
        let mut ids: [Option<String>; ANCESTOR_SLOTS] = Default::default();
        let mut versions = [None; ANCESTOR_SLOTS];
        for ((id, version), ancestor) in ids.iter_mut().zip(versions.iter_mut()).zip(ancestors) {
            *id = Some(ancestor.timeline_id.clone());
            *version = Some(to_i64(ancestor.max_version)?);
        }
        Ok(Self {
            timeline_id: timeline_id.to_owned(),
            ids,
            versions,
            axis: None,
        })
    }

    /// The binding of a positioned read: the scope's timelines and axis.
    /// Reads without a position use `new`, which binds no axis.
    fn of(scope: &TimelineScope) -> Result<Self> {
        Ok(Self::new(&scope.timeline_id, &scope.ancestors)?.on_axis(scope.axis.as_deref()))
    }

    fn on_axis(mut self, axis: Option<&str>) -> Self {
        self.axis = axis.map(str::to_owned);
        self
    }

    /// `:axis`, when the read is on an additional axis.
    fn axis_parameter(&self) -> Option<(&'static str, &dyn ToSql)> {
        self.axis.as_ref().map(|axis| (":axis", axis as &dyn ToSql))
    }

    /// `:timeline_id` and every ancestor slot.
    fn parameters(&self) -> [(&'static str, &dyn ToSql); 1 + 2 * ANCESTOR_SLOTS] {
        let [a1, a2, a3, a4] = &self.ids;
        let [v1, v2, v3, v4] = &self.versions;
        [
            (":timeline_id", &self.timeline_id),
            (":a1", a1),
            (":v1", v1),
            (":a2", a2),
            (":v2", v2),
            (":a3", a3),
            (":v3", v3),
            (":a4", a4),
            (":v4", v4),
        ]
    }

    /// Named parameters of a query that applies `visible!`, followed by the
    /// scope's own.
    fn with<'a>(
        &'a self,
        parameters: &[(&'static str, &'a dyn ToSql)],
    ) -> Vec<(&'static str, &'a dyn ToSql)> {
        let mut all = parameters.to_vec();
        all.extend(self.parameters());
        all.extend(self.axis_parameter());
        all
    }
}

/// A draft nested `MAX_DRAFT_DEPTH` deep, for query plans.
fn deepest_scope() -> TimelineScope {
    let mut ancestors = (1..MAX_DRAFT_DEPTH)
        .rev()
        .map(|level| ScopeAncestor::new(format!("draft-{level}"), Version::MAX / 2))
        .collect::<Vec<_>>();
    ancestors.push(ScopeAncestor::new(MAIN_TIMELINE_ID, Version::MAX / 2));
    TimelineScope::nested(format!("draft-{MAX_DRAFT_DEPTH}"), ancestors)
}

/// Main's head version and highest tick.
fn main_window(connection: &Connection, project_id: &str) -> Result<(Version, i64)> {
    let (version, max_tick): (i64, i64) = connection
        .prepare_cached("SELECT version, max_tick FROM projects WHERE id = ?1")
        .map_err(database_error)?
        .query_row([project_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| UmweltError::not_found(format!("Project '{project_id}' does not exist.")))?;
    Ok((to_u64(version)?, max_tick))
}

const TIMELINE_COLUMNS: &str = "id, parent_id, kind, status, base_version, head_version, max_tick, label, created_at, closed_at, purged_at";

type TimelineRow = (
    String,
    String,
    String,
    String,
    Option<i64>,
    i64,
    i64,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

fn timeline_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimelineRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
    ))
}

/// A draft from its row; its depth is the length of its ancestry.
fn timeline_from_row(
    connection: &Connection,
    project_id: &str,
    row: TimelineRow,
) -> Result<Timeline> {
    let (id, parent, kind, status, base, head, max_tick, label, created_at, closed_at, purged_at) =
        row;
    let base_version = base.map(to_u64).transpose()?;
    let depth = ancestry(
        connection,
        project_id,
        &parent,
        base_version.unwrap_or_default(),
    )?
    .len();
    Ok(Timeline {
        project_id: project_id.to_owned(),
        id,
        kind: from_json(&format!("\"{kind}\""))?,
        status: from_json(&format!("\"{status}\""))?,
        parent_timeline_id: Some(parent),
        depth: u32::try_from(depth).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Timeline depth overflowed.")
        })?,
        base_version,
        head_version: to_u64(head)?,
        max_tick,
        label,
        created_at,
        closed_at,
        purged_at,
    })
}

/// The ancestors of a draft whose parent is `parent_id` and whose base is
/// `base_version`: nearest first and ending with main, each with the highest
/// version of its rows the draft sees (see `TimelineScope`).
fn ancestry(
    connection: &Connection,
    project_id: &str,
    parent_id: &str,
    base_version: Version,
) -> Result<Vec<ScopeAncestor>> {
    let mut ancestors = vec![ScopeAncestor::new(parent_id, base_version)];
    let mut current = parent_id.to_owned();
    while current != MAIN_TIMELINE_ID {
        if ancestors.len() > ANCESTOR_SLOTS {
            return Err(UmweltError::public(
                UmweltErrorCode::InvalidState,
                format!("Timeline '{current}' is nested deeper than {MAX_DRAFT_DEPTH} drafts."),
            ));
        }
        let (parent, base): (String, Option<i64>) = connection
            .prepare_cached(
                "SELECT parent_id, base_version FROM timelines WHERE project_id = ?1 AND id = ?2",
            )
            .map_err(database_error)?
            .query_row(params![project_id, current], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                UmweltError::not_found(format!("Timeline '{current}' does not exist."))
            })?;
        ancestors.push(ScopeAncestor::new(
            parent.as_str(),
            to_u64(base.unwrap_or_default())?,
        ));
        current = parent;
    }
    Ok(ancestors)
}

/// The read and write scope of a timeline.
fn scope_of(connection: &Connection, timeline: &Timeline) -> Result<TimelineScope> {
    match &timeline.parent_timeline_id {
        None => Ok(TimelineScope::main()),
        Some(parent) => Ok(TimelineScope::nested(
            timeline.id.as_str(),
            ancestry(
                connection,
                &timeline.project_id,
                parent,
                timeline.base_version.unwrap_or_default(),
            )?,
        )),
    }
}

/// Main is synthesized from the project row; drafts come from `timelines`.
fn timeline_from_connection(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
) -> Result<Option<Timeline>> {
    if timeline_id == MAIN_TIMELINE_ID {
        return connection
            .prepare_cached("SELECT version, max_tick, created_at FROM projects WHERE id = ?1")
            .map_err(database_error)?
            .query_row([project_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .optional()
            .map_err(database_error)?
            .map(|(head, max_tick, created_at)| {
                Ok(Timeline {
                    project_id: project_id.to_owned(),
                    id: MAIN_TIMELINE_ID.to_owned(),
                    kind: TimelineKind::Main,
                    status: TimelineStatus::Active,
                    parent_timeline_id: None,
                    depth: 0,
                    base_version: None,
                    head_version: to_u64(head)?,
                    max_tick,
                    label: None,
                    created_at,
                    closed_at: None,
                    purged_at: None,
                })
            })
            .transpose();
    }
    connection
        .prepare_cached(&format!(
            "SELECT {TIMELINE_COLUMNS} FROM timelines WHERE project_id = ?1 AND id = ?2"
        ))
        .map_err(database_error)?
        .query_row(params![project_id, timeline_id], timeline_row)
        .optional()
        .map_err(database_error)?
        .map(|row| timeline_from_row(connection, project_id, row))
        .transpose()
}

/// Head version and highest tick of a timeline: main's on the project row, a
/// draft's on its timeline row.
fn timeline_window(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
) -> Result<(Version, i64)> {
    if timeline_id == MAIN_TIMELINE_ID {
        return main_window(connection, project_id);
    }
    let (head, max_tick): (i64, i64) = connection
        .prepare_cached(
            "SELECT head_version, max_tick FROM timelines WHERE project_id = ?1 AND id = ?2",
        )
        .map_err(database_error)?
        .query_row(params![project_id, timeline_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            UmweltError::not_found(format!("Timeline '{timeline_id}' does not exist."))
        })?;
    Ok((to_u64(head)?, max_tick))
}

/// Drafts below `timeline_id` that still depend on its rows, with their
/// status: every descendant that was neither purged nor promoted (a promoted
/// draft's rows already live in its parent). `active_only` keeps the active
/// ones.
fn dependent_descendants(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
    active_only: bool,
) -> Result<Vec<String>> {
    let mut statement = connection
        .prepare_cached(
            "WITH RECURSIVE below(id) AS ( \
             SELECT id FROM timelines WHERE project_id = ?1 AND parent_id = ?2 \
             AND purged_at IS NULL AND status <> 'promoted' \
             UNION SELECT t.id FROM timelines t JOIN below b ON t.parent_id = b.id \
             WHERE t.project_id = ?1 AND t.purged_at IS NULL AND t.status <> 'promoted') \
             SELECT t.id FROM timelines t JOIN below b ON t.id = b.id \
             WHERE t.project_id = ?1 AND (?3 = 0 OR t.status = 'active') ORDER BY t.id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![project_id, timeline_id, active_only], |row| {
            row.get::<_, String>(0)
        })
        .map_err(database_error)?;
    rows.map(|row| row.map_err(database_error)).collect()
}

/// Refuses `operation` on a draft that other drafts still depend on.
fn require_no_descendants(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
    active_only: bool,
    operation: &str,
) -> Result<()> {
    let descendants = dependent_descendants(connection, project_id, timeline_id, active_only)?;
    if descendants.is_empty() {
        return Ok(());
    }
    let remedy = if active_only {
        "Discard them first."
    } else {
        "Discard and purge them first."
    };
    Err(UmweltError::public(
        UmweltErrorCode::InvalidState,
        format!(
            "Timeline '{timeline_id}' cannot be {operation}: drafts opened from it depend on it. \
             {remedy}"
        ),
    )
    .with_details(json!({ "timelineId": timeline_id, "descendants": descendants })))
}

fn existing_draft(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
) -> Result<Timeline> {
    let timeline =
        timeline_from_connection(connection, project_id, timeline_id)?.ok_or_else(|| {
            UmweltError::not_found(format!("Timeline '{timeline_id}' does not exist."))
        })?;
    if timeline.kind == TimelineKind::Main {
        return Err(UmweltError::public(
            UmweltErrorCode::InvalidState,
            "The main timeline cannot be promoted, discarded or purged.",
        ));
    }
    Ok(timeline)
}

/// The parent of a draft.
fn parent_of(timeline: &Timeline) -> Result<&str> {
    timeline.parent_timeline_id.as_deref().ok_or_else(|| {
        UmweltError::public(
            UmweltErrorCode::InvalidState,
            format!("Timeline '{}' has no parent.", timeline.id),
        )
    })
}

fn require_active(timeline: &Timeline) -> Result<()> {
    if timeline.status == TimelineStatus::Active {
        return Ok(());
    }
    Err(UmweltError::public(
        UmweltErrorCode::InvalidState,
        format!("Timeline '{}' is not active.", timeline.id),
    )
    .with_details(json!({ "timelineId": timeline.id, "status": timeline.status })))
}

/// Deletes every row a discarded draft wrote. Main rows are never touched:
/// each statement is restricted to the draft's own timeline id.
fn purge_rows(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
    purged_at: &str,
) -> Result<PurgeCounts> {
    for sql in PURGE_EMBEDDINGS {
        connection
            .execute(sql, params![project_id, timeline_id])
            .map_err(database_error)?;
    }
    let counts = delete_draft_rows(connection, project_id, timeline_id)?;
    connection
        .execute(
            "UPDATE timelines SET purged_at = ?3 WHERE project_id = ?1 AND id = ?2",
            params![project_id, timeline_id, purged_at],
        )
        .map_err(database_error)?;
    Ok(counts)
}

/// Deletes every row of a draft, keeping its timeline row.
fn delete_draft_rows(
    connection: &Connection,
    project_id: &str,
    timeline_id: &str,
) -> Result<PurgeCounts> {
    let run = |sql: &str| -> Result<u64> {
        let changed = connection
            .execute(sql, params![project_id, timeline_id])
            .map_err(database_error)?;
        u64::try_from(changed).map_err(|_| {
            UmweltError::public(UmweltErrorCode::InvalidState, "Row count overflowed.")
        })
    };
    for sql in PURGE_STATEMENTS {
        run(sql)?;
    }
    run(PURGE_CHANGE_FEED)?;
    let counts = PurgeCounts {
        transitions: run(PURGE_TRANSITIONS)?,
        records: run(PURGE_RECORDS)?,
        events: run(PURGE_EVENTS)?,
        attempts: run(PURGE_ATTEMPTS)?,
    };
    Ok(counts)
}

/// One commit of a draft, rebuilt from its stored rows so it can be applied
/// again: same ids, same attempt and fingerprint, same preconditions.
fn stored_draft_commits(
    connection: &Connection,
    provenances: &ProvenanceCache,
    project_id: &str,
    timeline_id: &str,
) -> Result<Vec<PreparedCommit>> {
    let mut statement = connection
        .prepare(
            "SELECT attempt_id, fingerprint, event_id, recorded_version, extra FROM attempts \
             WHERE project_id = ?1 AND timeline_id = ?2 ORDER BY recorded_version",
        )
        .map_err(database_error)?;
    let attempts = statement
        .query_map(params![project_id, timeline_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    // The draft's own rows only: no ancestor row is visible at version 0.
    let scope = TimelineScope::draft(timeline_id, 0);
    let missing = |what: &str, id: &str| {
        UmweltError::public(
            UmweltErrorCode::InvalidState,
            format!("Draft {what} '{id}' is missing."),
        )
    };
    let mut commits = Vec::with_capacity(attempts.len());
    for (attempt_id, fingerprint, event_id, version, extra) in attempts {
        let stored = stored_attempt_from(fingerprint, event_id, version, extra)?;
        let outcome = stored.outcome;
        let event = event_from_connection(
            connection,
            provenances,
            project_id,
            &scope,
            &outcome.event_id,
        )?
        .ok_or_else(|| missing("event", &outcome.event_id))?;
        let mut records = Vec::with_capacity(outcome.record_ids.len());
        for id in &outcome.record_ids {
            let record = record_from_connection(connection, provenances, project_id, &scope, id)?
                .ok_or_else(|| missing("record", id))?;
            records.push(record_input(record));
        }
        let mut transitions = Vec::with_capacity(outcome.transition_ids.len());
        for id in &outcome.transition_ids {
            let transition = transition_rows(
                connection,
                provenances,
                &format!(
                    "SELECT {TRANSITION_COLUMNS} FROM transitions t \
                     WHERE t.project_id = ?1 AND t.id = ?2"
                ),
                params![project_id, id],
            )?
            .pop()
            .ok_or_else(|| missing("transition", id))?;
            transitions.push(TransitionInput {
                id: Some(transition.id),
                target_record_id: transition.target_record_id,
                status: transition.status,
                superseded_by: transition.superseded_by,
                position: Some(transition.position),
                provenance: Some(transition.provenance),
            });
        }
        commits.push(PreparedCommit {
            input: CommitInput {
                project_id: project_id.to_owned(),
                attempt_id,
                timeline_id: Some(timeline_id.to_owned()),
                expected_version: None,
                event: EventInput {
                    id: Some(event.id),
                    event_type: event.event_type,
                    title: event.title,
                    description: event.description,
                    position: event.position,
                    on: event.on,
                    participants: event.participants,
                    location: event.location,
                    evidence: event.evidence,
                    provenance: event.provenance,
                },
                records,
                transitions,
                preconditions: stored.preconditions,
            },
            fingerprint: stored.fingerprint,
            event_id: outcome.event_id,
            record_ids: outcome.record_ids,
            transition_ids: outcome.transition_ids,
        });
    }
    Ok(commits)
}

fn record_input(record: Record) -> RecordInput {
    let input_common = |common: RecordCommon| RecordInputCommon {
        id: Some(common.id),
        position: Some(common.position),
        provenance: Some(common.provenance),
    };
    match record {
        Record::Fact { common, claim } => RecordInput::Fact {
            common: input_common(common),
            claim,
        },
        Record::Holding {
            common,
            holder,
            attitude,
            claim,
            acquisition,
            confidence,
            accessibility,
            text,
            secrecy,
        } => RecordInput::Holding {
            common: input_common(common),
            holder,
            attitude,
            claim,
            acquisition,
            confidence,
            accessibility,
            text,
            secrecy,
        },
        Record::Awareness {
            common,
            holder,
            target,
        } => RecordInput::Awareness {
            common: input_common(common),
            holder,
            target,
        },
        Record::Disposition {
            common,
            holder,
            target,
            dimension,
            value,
        } => RecordInput::Disposition {
            common: input_common(common),
            holder,
            target,
            dimension,
            value,
        },
    }
}

fn proposal_from_connection(
    connection: &Connection,
    proposal_id: &str,
) -> Result<Option<Proposal>> {
    query_json_optional(
        connection,
        "SELECT payload FROM proposals WHERE id = ?1",
        [proposal_id],
    )
}

fn update_proposal(connection: &Connection, proposal: &Proposal) -> Result<()> {
    connection
        .execute(
            "UPDATE proposals SET status = ?2, rationale = ?3, payload = ?4 WHERE id = ?1",
            params![
                proposal.id,
                status_name(proposal.status),
                proposal.rejection_rationale,
                to_json(proposal)?
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn is_current_window(
    connection: &Connection,
    project_id: &str,
    tick: i64,
    as_of_version: Version,
) -> Result<bool> {
    let (project_version, maximum_tick): (i64, i64) = connection
        .prepare_cached("SELECT version, max_tick FROM projects WHERE id = ?1")
        .map_err(database_error)?
        .query_row([project_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(database_error)?;
    Ok(to_u64(project_version)? == as_of_version && tick >= maximum_tick)
}

fn search_records(
    connection: &Connection,
    provenances: &ProvenanceCache,
    query: &SearchQuery,
) -> Result<SearchPage> {
    if query.semantic.is_some() {
        return ranked_search(connection, provenances, query);
    }
    let safe_text = safe_fts_query(&query.text);
    if safe_text.is_empty() {
        return Ok(SearchPage {
            items: Vec::new(),
            next_offset: None,
        });
    }
    let limit = i64::from(query.limit.clamp(1, 1000)) + 1;
    let offset = to_i64(query.offset)?;
    let version = to_i64(query.as_of_version)?;
    // Columns: doc type, doc id, then a record (RECORD_COLUMNS, possibly all
    // NULL) and an event (EVENT_COLUMNS, possibly all NULL).
    let columns = if query.holder_id.is_some() {
        format!(
            "f.doc_type, f.doc_id, {RECORD_COLUMNS}, {}",
            ["NULL"; rows::EVENT_COLUMN_COUNT].join(", ")
        )
    } else {
        format!("f.doc_type, f.doc_id, {RECORD_COLUMNS}, {EVENT_COLUMNS}")
    };
    let sql = text_search_sql(query.holder_id.is_some(), &columns);
    let sql = bound_on_axis(&sql, query.axis.is_some());
    let scope =
        ScopeBinding::new(&query.timeline_id, &query.ancestors)?.on_axis(query.axis.as_deref());
    let mut parameters = scope.with(&[
        (":project_id", &query.project_id),
        (":text", &safe_text),
        (":doc_type", &query.doc_type),
        (":tick", &query.tick),
        (":version", &version),
        (":limit", &limit),
        (":offset", &offset),
    ]);
    if let Some(holder) = &query.holder_id {
        parameters.push((":holder", holder));
    }
    let mut statement = connection.prepare_cached(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(&parameters[..], |row| {
            let doc_type: String = row.get(0)?;
            let doc_id: String = row.get(1)?;
            let record = if doc_type == RECORD_DOC {
                Some(RecordRow::read(row, 2)?)
            } else {
                None
            };
            let event = if doc_type == EVENT_DOC {
                Some(EventRow::read(row, 2 + RECORD_COLUMN_COUNT)?)
            } else {
                None
            };
            Ok((doc_type, doc_id, record, event))
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    let page_limit = usize::try_from(limit - 1).map_err(|_| {
        UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
    })?;
    let has_more = rows.len() > page_limit;
    let mut items = Vec::with_capacity(rows.len().min(page_limit));
    for (doc_type, doc_id, record, event) in rows.into_iter().take(page_limit) {
        items.push(SearchItem {
            doc_type,
            doc_id,
            record: record
                .map(|row| row.into_record(connection, provenances))
                .transpose()?,
            event: event
                .map(|row| row.into_event(connection, provenances))
                .transpose()?,
            matched: Some(SearchMatch::Text),
            score: None,
        });
    }
    Ok(SearchPage {
        items,
        next_offset: has_more.then_some(query.offset + u64::from(query.limit.clamp(1, 1000))),
    })
}

/// Full-text search, with the holder branch (the holder's active, available
/// Holdings) or the narrator branch (active Facts and visible events).
fn text_search_sql(holder: bool, columns: &str) -> String {
    if holder {
        format!(
            "SELECT {columns} FROM search_fts f \
             JOIN records r ON r.project_id = f.project_id AND r.id = f.doc_id \
             WHERE search_fts MATCH :text AND f.project_id = :project_id AND f.doc_type = 'record' \
             AND r.kind = 'holding' AND r.holder_id = :holder \
             AND r.accessibility = 'available' AND r.tick <= :tick \
             AND r.recorded_version <= :version AND {visible_r} \
             AND (:doc_type IS NULL OR f.doc_type = :doc_type) \
             AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id \
             AND t.target_id = r.id AND t.tick <= :tick AND t.recorded_version <= :version \
             AND {visible_t}) \
             ORDER BY rank, f.doc_type, f.doc_id LIMIT :limit OFFSET :offset",
            visible_r = visible!("r"),
            visible_t = visible!("t"),
        )
    } else {
        format!(
            "SELECT {columns} FROM search_fts f \
             LEFT JOIN records r ON r.project_id = f.project_id AND r.id = f.doc_id \
             AND f.doc_type = 'record' \
             LEFT JOIN events e ON e.project_id = f.project_id AND e.id = f.doc_id \
             AND f.doc_type = 'event' \
             WHERE search_fts MATCH :text AND f.project_id = :project_id \
             AND (:doc_type IS NULL OR f.doc_type = :doc_type) \
             AND ((f.doc_type = 'event' AND e.tick <= :tick AND e.recorded_version <= :version \
             AND {visible_e}) \
             OR (f.doc_type = 'record' AND r.kind = 'fact' AND r.tick <= :tick \
             AND r.recorded_version <= :version AND {visible_r} \
             AND NOT EXISTS (SELECT 1 FROM transitions t \
             WHERE t.project_id = r.project_id AND t.target_id = r.id \
             AND t.tick <= :tick AND t.recorded_version <= :version AND {visible_t}))) \
             ORDER BY rank, f.doc_type, f.doc_id LIMIT :limit OFFSET :offset",
            visible_e = visible!("e"),
            visible_r = visible!("r"),
            visible_t = visible!("t"),
        )
    }
}

/// Vectors of the documents a search may return, under the same conditions
/// as `text_search_sql`. With a holder the scan starts from the holder's
/// Holdings (`claims_holder_kind_order`) and reaches each vector by its key;
/// without one it reads the space's vectors and checks each document.
const SEMANTIC_HOLDER: &str = concat!(
    "SELECT 'record', r.id, m.vector FROM records r INDEXED BY claims_holder_kind_order \
     CROSS JOIN embeddings m ON m.project_id = r.project_id AND m.space_id = :space_id \
     AND m.doc_type = 'record' AND m.doc_id = r.id \
     WHERE r.project_id = :project_id AND r.holder_id = :holder AND r.kind = 'holding' \
     AND r.accessibility = 'available' AND r.tick <= :tick AND r.recorded_version <= :version \
     AND (:doc_type IS NULL OR :doc_type = 'record') AND ",
    visible!("r"),
    " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id \
     AND t.target_id = r.id AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    ")"
);
const SEMANTIC_NARRATOR: &str = concat!(
    "SELECT m.doc_type, m.doc_id, m.vector FROM embeddings m \
     LEFT JOIN records r ON r.project_id = m.project_id AND r.id = m.doc_id \
     AND m.doc_type = 'record' \
     LEFT JOIN events e ON e.project_id = m.project_id AND e.id = m.doc_id \
     AND m.doc_type = 'event' \
     WHERE m.project_id = :project_id AND m.space_id = :space_id \
     AND (:doc_type IS NULL OR m.doc_type = :doc_type) \
     AND ((m.doc_type = 'event' AND e.tick <= :tick AND e.recorded_version <= :version AND ",
    visible!("e"),
    ") OR (m.doc_type = 'record' AND r.kind = 'fact' AND r.tick <= :tick \
     AND r.recorded_version <= :version AND ",
    visible!("r"),
    " AND NOT EXISTS (SELECT 1 FROM transitions t WHERE t.project_id = r.project_id \
     AND t.target_id = r.id AND t.tick <= :tick AND t.recorded_version <= :version AND ",
    visible!("t"),
    ")))"
);

/// A semantic or hybrid search: the best `offset + limit` documents of each
/// ranking, fused when both are present, then loaded.
fn ranked_search(
    connection: &Connection,
    provenances: &ProvenanceCache,
    query: &SearchQuery,
) -> Result<SearchPage> {
    let Some(semantic) = &query.semantic else {
        return Ok(SearchPage {
            items: Vec::new(),
            next_offset: None,
        });
    };
    let limit = u64::from(query.limit.clamp(1, 1000));
    if query.offset + limit > MAX_SEMANTIC_WINDOW {
        return Err(UmweltError::validation(format!(
            "A semantic search reads at most {MAX_SEMANTIC_WINDOW} results (offset + limit)."
        )));
    }
    // One more than the page, to know whether another page follows.
    let window = usize::try_from(query.offset + limit + 1).map_err(|_| {
        UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
    })?;
    let scored = semantic_ranking(connection, query, semantic, window)?;
    let safe_text = safe_fts_query(&query.text);
    let ranked = if safe_text.is_empty() {
        scored
            .into_iter()
            .map(|(doc, score)| RankedDoc {
                doc,
                matched: SearchMatch::Semantic,
                score: Some(score),
            })
            .collect::<Vec<_>>()
    } else {
        let text = text_ranking(connection, query, &safe_text, window)?;
        fuse_rankings(&text, &scored)
    };
    let start = usize::try_from(query.offset).map_err(|_| {
        UmweltError::public(UmweltErrorCode::InvalidState, "Query offset is invalid.")
    })?;
    let end = start + usize::try_from(limit).unwrap_or(1000);
    let has_more = ranked.len() > end;
    let scope = TimelineScope::nested(query.timeline_id.as_str(), query.ancestors.clone());
    let mut items = Vec::new();
    for hit in ranked.into_iter().skip(start).take(end - start) {
        let (record, event) = if hit.doc.doc_type == RECORD_DOC {
            let record = record_from_connection(
                connection,
                provenances,
                &query.project_id,
                &scope,
                &hit.doc.doc_id,
            )?;
            (record, None)
        } else {
            let event = event_from_connection(
                connection,
                provenances,
                &query.project_id,
                &scope,
                &hit.doc.doc_id,
            )?;
            (None, event)
        };
        items.push(SearchItem {
            doc_type: hit.doc.doc_type,
            doc_id: hit.doc.doc_id,
            record,
            event,
            matched: Some(hit.matched),
            score: hit.score,
        });
    }
    Ok(SearchPage {
        items,
        next_offset: has_more.then_some(query.offset + limit),
    })
}

/// The best `window` documents by cosine similarity, best first, ties broken
/// by document type and id. Vectors are scored as they stream from SQLite;
/// only the best `window` are kept.
fn semantic_ranking(
    connection: &Connection,
    query: &SearchQuery,
    semantic: &SemanticQuery,
    window: usize,
) -> Result<Vec<(DocRef, f64)>> {
    let sql = bound_on_axis(
        if query.holder_id.is_some() {
            SEMANTIC_HOLDER
        } else {
            SEMANTIC_NARRATOR
        },
        query.axis.is_some(),
    );
    let version = to_i64(query.as_of_version)?;
    let scope =
        ScopeBinding::new(&query.timeline_id, &query.ancestors)?.on_axis(query.axis.as_deref());
    let mut parameters = scope.with(&[
        (":project_id", &query.project_id),
        (":space_id", &semantic.space_id),
        (":doc_type", &query.doc_type),
        (":tick", &query.tick),
        (":version", &version),
    ]);
    if let Some(holder) = &query.holder_id {
        parameters.push((":holder", holder));
    }
    let mut statement = connection.prepare_cached(&sql).map_err(database_error)?;
    let mut rows = statement.query(&parameters[..]).map_err(database_error)?;
    // The worst kept result sits on top of the heap.
    let mut best: BinaryHeap<Ranked> = BinaryHeap::with_capacity(window + 1);
    while let Some(row) = rows.next().map_err(database_error)? {
        let vector = rows::blob_vector(&row.get::<_, Vec<u8>>(2).map_err(database_error)?)?;
        if vector.len() != semantic.vector.len() {
            continue;
        }
        best.push(Ranked((
            DocRef {
                doc_type: row.get(0).map_err(database_error)?,
                doc_id: row.get(1).map_err(database_error)?,
            },
            similarity(&semantic.vector, &vector),
        )));
        if best.len() > window {
            best.pop();
        }
    }
    let mut ranked = best.into_iter().map(|item| item.0).collect::<Vec<_>>();
    ranked.sort_by(rank_order);
    Ok(ranked)
}

/// A result ordered so that the heap's top is the worst one (`rank_order`
/// sorts best first).
struct Ranked((DocRef, f64));

impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        rank_order(&self.0, &other.0) == std::cmp::Ordering::Equal
    }
}
impl Eq for Ranked {}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        rank_order(&self.0, &other.0)
    }
}

/// The best `window` documents of the full-text search, best first.
fn text_ranking(
    connection: &Connection,
    query: &SearchQuery,
    safe_text: &str,
    window: usize,
) -> Result<Vec<DocRef>> {
    let sql = text_search_sql(query.holder_id.is_some(), "f.doc_type, f.doc_id");
    let sql = bound_on_axis(&sql, query.axis.is_some());
    let version = to_i64(query.as_of_version)?;
    let limit = i64::try_from(window).unwrap_or(i64::MAX);
    let offset = 0_i64;
    let scope =
        ScopeBinding::new(&query.timeline_id, &query.ancestors)?.on_axis(query.axis.as_deref());
    let mut parameters = scope.with(&[
        (":project_id", &query.project_id),
        (":text", &safe_text),
        (":doc_type", &query.doc_type),
        (":tick", &query.tick),
        (":version", &version),
        (":limit", &limit),
        (":offset", &offset),
    ]);
    if let Some(holder) = &query.holder_id {
        parameters.push((":holder", holder));
    }
    let mut statement = connection.prepare_cached(&sql).map_err(database_error)?;
    let rows = statement
        .query_map(&parameters[..], |row| {
            Ok(DocRef {
                doc_type: row.get(0)?,
                doc_id: row.get(1)?,
            })
        })
        .map_err(database_error)?;
    rows.map(|row| row.map_err(database_error)).collect()
}

/// Searchable documents visible in a timeline whose vector in a space is
/// missing or stale (its text hash is not the document's), in the order they
/// were indexed (`search_fts` rowid).
const PENDING_EMBEDDINGS: &str = concat!(
    "SELECT f.rowid, f.doc_type, f.doc_id, f.text, m.doc_id IS NOT NULL FROM search_fts f \
     LEFT JOIN records r ON f.doc_type = 'record' AND r.project_id = f.project_id \
     AND r.id = f.doc_id \
     LEFT JOIN events e ON f.doc_type = 'event' AND e.project_id = f.project_id \
     AND e.id = f.doc_id \
     LEFT JOIN embeddings m ON m.project_id = f.project_id AND m.space_id = :space_id \
     AND m.doc_type = f.doc_type AND m.doc_id = f.doc_id \
     WHERE f.rowid > :after AND f.project_id = :project_id \
     AND ((f.doc_type = 'record' AND :records AND ",
    visible!("r"),
    ") OR (f.doc_type = 'event' AND :events AND ",
    visible!("e"),
    ")) AND (m.doc_id IS NULL OR m.text_hash <> umwelt_text_hash(f.text)) \
     ORDER BY f.rowid LIMIT :limit"
);

fn pending_embeddings(connection: &Connection, query: &PendingQuery) -> Result<PendingPage> {
    let records = query.doc_types.is_empty() || query.doc_types.iter().any(|t| t == RECORD_DOC);
    let events = query.doc_types.is_empty() || query.doc_types.iter().any(|t| t == EVENT_DOC);
    let limit = i64::from(query.limit.clamp(1, 1000)) + 1;
    let scope = ScopeBinding::new(&query.timeline_id, &query.ancestors)?;
    let parameters = scope.with(&[
        (":project_id", &query.project_id),
        (":space_id", &query.space_id),
        (":after", &query.after),
        (":records", &records),
        (":events", &events),
        (":limit", &limit),
    ]);
    let mut statement = connection
        .prepare_cached(PENDING_EMBEDDINGS)
        .map_err(database_error)?;
    let rows = statement
        .query_map(&parameters[..], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    let page_limit = usize::try_from(limit - 1).map_err(|_| {
        UmweltError::public(UmweltErrorCode::InvalidState, "Query limit is invalid.")
    })?;
    let has_more = rows.len() > page_limit;
    let mut last = query.after;
    let mut items = Vec::with_capacity(rows.len().min(page_limit));
    for (rowid, doc_type, doc_id, text, stale) in rows.into_iter().take(page_limit) {
        last = rowid;
        items.push(PendingEmbedding {
            text_hash: text_hash(&text),
            doc_type,
            doc_id,
            text,
            stale,
        });
    }
    if has_more {
        return Ok(PendingPage {
            items,
            next: Some(last),
            resume: last,
        });
    }
    // Every document indexed so far was checked.
    let newest: Option<i64> = connection
        .prepare_cached("SELECT MAX(rowid) FROM search_fts")
        .map_err(database_error)?
        .query_row([], |row| row.get(0))
        .map_err(database_error)?;
    Ok(PendingPage {
        items,
        next: None,
        resume: newest.unwrap_or(0).max(last),
    })
}

/// A searchable document's current text, read by id in any timeline:
/// `None` when it exists but is not searchable. Fails with `NOT_FOUND` when it
/// does not exist.
fn document_text(
    connection: &Connection,
    provenances: &ProvenanceCache,
    project_id: &str,
    doc_type: &str,
    doc_id: &str,
) -> Result<Option<String>> {
    let missing =
        || UmweltError::not_found(format!("Document {doc_type} '{doc_id}' does not exist."));
    if doc_type == RECORD_DOC {
        let record = record_rows(
            connection,
            provenances,
            concat!(
                "SELECT ",
                record_columns!(),
                " FROM records r WHERE r.project_id = ?1 AND r.id = ?2"
            ),
            params![project_id, doc_id],
        )?
        .pop()
        .ok_or_else(missing)?;
        Ok(record_search_text(&record))
    } else {
        let row = connection
            .prepare_cached(concat!(
                "SELECT ",
                event_columns!(),
                " FROM events e WHERE e.project_id = ?1 AND e.id = ?2"
            ))
            .map_err(database_error)?
            .query_row(params![project_id, doc_id], |row| EventRow::read(row, 0))
            .optional()
            .map_err(database_error)?
            .ok_or_else(missing)?;
        Ok(event_search_text(&row.into_event(connection, provenances)?))
    }
}

fn embedding_space(
    connection: &Connection,
    project_id: &str,
    space_id: &str,
) -> Result<Option<EmbeddingSpace>> {
    connection
        .prepare_cached(
            "SELECT id, dimensions, model FROM embedding_spaces WHERE project_id = ?1 AND id = ?2",
        )
        .map_err(database_error)?
        .query_row(params![project_id, space_id], |row| {
            Ok(EmbeddingSpace {
                id: row.get(0)?,
                dimensions: row.get(1)?,
                model: row.get(2)?,
            })
        })
        .optional()
        .map_err(database_error)
}

fn require_embedding_space(
    connection: &Connection,
    project_id: &str,
    space_id: &str,
) -> Result<EmbeddingSpace> {
    embedding_space(connection, project_id, space_id)?.ok_or_else(|| {
        UmweltError::not_found(format!("Embedding space '{space_id}' does not exist."))
    })
}

fn safe_fts_query(text: &str) -> String {
    text.split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn query_json_optional<T: DeserializeOwned, P: rusqlite::Params>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> Result<Option<T>> {
    let mut statement = connection.prepare_cached(sql).map_err(database_error)?;
    statement
        .query_row(params, |row| row.get::<_, String>(0))
        .optional()
        .map_err(database_error)?
        .map(|json| from_json(&json))
        .transpose()
}

fn count(connection: &Connection, table: &str, project_id: &str) -> Result<u64> {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE project_id = ?1");
    let value: i64 = connection
        .query_row(&sql, [project_id], |row| row.get(0))
        .map_err(database_error)?;
    to_u64(value)
}

fn status_name(status: ProposalStatus) -> &'static str {
    match status {
        ProposalStatus::Pending => "pending",
        ProposalStatus::Applied => "applied",
        ProposalStatus::PartiallyApplied => "partially-applied",
        ProposalStatus::Rejected => "rejected",
    }
}

fn to_json(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

fn enum_name(value: &impl Serialize) -> Result<String> {
    let encoded = to_json(value)?;
    Ok(encoded.trim_matches('"').to_owned())
}

fn from_json<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(Into::into)
}

fn to_i64(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| {
        UmweltError::public(
            UmweltErrorCode::InvalidState,
            "Version exceeds SQLite integer range.",
        )
    })
}

fn to_u64(value: i64) -> Result<u64> {
    u64::try_from(value).map_err(|_| {
        UmweltError::public(UmweltErrorCode::InvalidState, "Stored version is negative.")
    })
}

/// Which constraint an insert broke: the row's primary key, or another unique
/// key of the table.
enum Duplicate {
    PrimaryKey,
    Unique,
}

/// Turns an insert that broke a primary or unique key into a `CONFLICT` with
/// `message`; any other failure stays a `DATABASE_ERROR`.
fn duplicate_or_database_error(
    source: rusqlite::Error,
    message: impl FnOnce(Duplicate) -> String,
) -> UmweltError {
    let duplicate = match &source {
        rusqlite::Error::SqliteFailure(error, _) => match error.extended_code {
            rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY => Some(Duplicate::PrimaryKey),
            rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE => Some(Duplicate::Unique),
            _ => None,
        },
        _ => None,
    };
    match duplicate {
        Some(duplicate) => UmweltError::public(UmweltErrorCode::Conflict, message(duplicate)),
        None => database_error(source),
    }
}

fn database_error(source: rusqlite::Error) -> UmweltError {
    UmweltError::internal(
        UmweltErrorCode::DatabaseError,
        "SQLite operation failed.",
        source,
    )
}
