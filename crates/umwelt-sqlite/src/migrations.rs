// SPDX-License-Identifier: AGPL-3.0-only
//! Converts files of an earlier storage version (see `spec/versioning.md`).
//!
//! Every step moves a file from one version to the next. A migration copies
//! the file to a backup next to it, then runs every step it needs in one
//! transaction, so a failure leaves the original untouched.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, Transaction, TransactionBehavior};
use serde_json::json;
use umwelt_core::domain::StorageMigration;
use umwelt_core::{Result, UmweltError, UmweltErrorCode};

use crate::database_error;

/// Converts an open file from storage version `from` to `from + 1`.
pub(crate) type Step = fn(&Transaction<'_>) -> Result<()>;

/// The oldest storage version that was ever published (0.1.0). Files of an
/// older version are refused.
pub(crate) const OLDEST_PUBLISHED: i64 = 5;

/// The steps, indexed by the version they start from.
pub(crate) const MIGRATIONS: &[(i64, Step)] = &[(5, v5_to_v6)];

/// Storage 6: an event's participants move from its `extra` JSON into
/// `event_participants`, in the order they were written, so reads can find
/// the events of one entity through an index.
fn v5_to_v6(transaction: &Transaction<'_>) -> Result<()> {
    transaction
        .execute_batch(
            "CREATE TABLE event_participants (
    project_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    entity_id TEXT NOT NULL,
    tick INTEGER NOT NULL,
    recorded_version INTEGER NOT NULL,
    PRIMARY KEY (project_id, event_id, ordinal)
) WITHOUT ROWID;
CREATE INDEX event_participants_order ON event_participants(project_id, entity_id, tick, recorded_version, event_id);
INSERT INTO event_participants (project_id, event_id, ordinal, entity_id, tick, recorded_version)
    SELECT e.project_id, e.id, p.key, p.value, e.tick, e.recorded_version
    FROM events e, json_each(e.extra, '$.participants') p
    WHERE e.extra IS NOT NULL;
UPDATE events SET extra = NULLIF(json_remove(extra, '$.participants'), '{}')
    WHERE extra IS NOT NULL AND json_type(extra, '$.participants') IS NOT NULL;",
        )
        .map_err(database_error)
}

/// Backs the file up and runs every step from `from` to `to`.
pub(crate) fn migrate(
    connection: &mut Connection,
    path: &Path,
    from: i64,
    to: i64,
    steps: &[(i64, Step)],
) -> Result<StorageMigration> {
    let backup = backup(connection, path, from)?;
    let failure = |error: UmweltError| {
        UmweltError::public(
            UmweltErrorCode::MigrationFailed,
            format!(
                "Could not migrate the file from storage version {from} to {to}: {}. \
                 The file was not changed; a copy is at '{}'.",
                error.message,
                backup.display()
            ),
        )
        .with_details(json!({
            "from": from,
            "to": to,
            "backup": backup.display().to_string(),
        }))
    };
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| failure(database_error(error)))?;
    for version in from..to {
        let step = steps
            .iter()
            .find(|(start, _)| *start == version)
            .map(|(_, step)| *step)
            .ok_or_else(|| {
                failure(UmweltError::public(
                    UmweltErrorCode::MigrationFailed,
                    format!("no step from storage version {version}"),
                ))
            })?;
        step(&transaction).map_err(failure)?;
    }
    transaction
        .pragma_update(None, "user_version", to)
        .map_err(|error| failure(database_error(error)))?;
    transaction
        .commit()
        .map_err(|error| failure(database_error(error)))?;
    checkpoint(connection).map_err(failure)?;
    Ok(StorageMigration {
        from: to_u32(from)?,
        to: to_u32(to)?,
        backup: backup.display().to_string(),
    })
}

/// Copies the file, byte for byte, to the first free backup name. The
/// caller holds the file's lock, so no other connection writes meanwhile.
fn backup(connection: &Connection, path: &Path, version: i64) -> Result<PathBuf> {
    checkpoint(connection)?;
    let target = backup_path(path, version);
    std::fs::copy(path, &target).map_err(|error| {
        UmweltError::internal(
            UmweltErrorCode::MigrationFailed,
            format!(
                "Could not back up the file to '{}' before migrating it.",
                target.display()
            ),
            error,
        )
    })?;
    Ok(target)
}

/// `story.umw.v5.bak`, or `story.umw.v5.2.bak`, `.v5.3.bak`... when taken.
/// An existing backup is never overwritten.
pub(crate) fn backup_path(path: &Path, version: i64) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let candidate = |suffix: String| path.with_file_name(format!("{name}.v{version}{suffix}.bak"));
    let first = candidate(String::new());
    if !first.exists() {
        return first;
    }
    (2_u32..)
        .map(|number| candidate(format!(".{number}")))
        .find(|path| !path.exists())
        .unwrap_or(first)
}

/// Moves every committed page from the WAL into the main file, so the main
/// file alone holds the whole database.
fn checkpoint(connection: &Connection) -> Result<()> {
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .map_err(database_error)?;
    if busy != 0 {
        return Err(UmweltError::public(
            UmweltErrorCode::MigrationFailed,
            "Another connection is using the file; it cannot be migrated now.",
        ));
    }
    Ok(())
}

fn to_u32(version: i64) -> Result<u32> {
    u32::try_from(version)
        .map_err(|_| UmweltError::validation(format!("Invalid storage version {version}.")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::SqliteStore;

    /// A file of the current layout, labelled one version older.
    fn older_file(directory: &Path) -> PathBuf {
        let path = directory.join("story.umw");
        drop(SqliteStore::create(&path).unwrap());
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", crate::FILE_VERSION - 1)
            .unwrap();
        path
    }

    fn mark(transaction: &Transaction<'_>) -> Result<()> {
        transaction
            .execute_batch("CREATE TABLE migrated (id INTEGER)")
            .map_err(database_error)
    }

    fn fail(transaction: &Transaction<'_>) -> Result<()> {
        mark(transaction)?;
        Err(UmweltError::validation("step failed on purpose"))
    }

    fn open(path: &Path) -> Connection {
        Connection::open(path).unwrap()
    }

    fn has_table(connection: &Connection, name: &str) -> bool {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

    #[test]
    fn a_migration_backs_up_the_original_and_converts_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = older_file(directory.path());
        let original = std::fs::read(&path).unwrap();
        let from = crate::FILE_VERSION - 1;
        let mut connection = open(&path);
        let migration = migrate(
            &mut connection,
            &path,
            from,
            crate::FILE_VERSION,
            &[(from, mark)],
        )
        .unwrap();
        assert_eq!(i64::from(migration.from), from);
        assert_eq!(i64::from(migration.to), crate::FILE_VERSION);
        let backup = PathBuf::from(&migration.backup);
        assert_eq!(
            backup.file_name().unwrap().to_string_lossy(),
            format!("story.umw.v{from}.bak")
        );
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        assert!(has_table(&connection, "migrated"));
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, crate::FILE_VERSION);
        assert!(!has_table(&open(&backup), "migrated"));
    }

    #[test]
    fn an_existing_backup_is_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("story.umw");
        std::fs::write(directory.path().join("story.umw.v5.bak"), b"first").unwrap();
        std::fs::write(directory.path().join("story.umw.v5.2.bak"), b"second").unwrap();
        assert_eq!(
            backup_path(&path, 5).file_name().unwrap().to_string_lossy(),
            "story.umw.v5.3.bak"
        );
    }

    #[test]
    fn a_failed_step_leaves_the_file_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let path = older_file(directory.path());
        let from = crate::FILE_VERSION - 1;
        let mut connection = open(&path);
        let error = migrate(
            &mut connection,
            &path,
            from,
            crate::FILE_VERSION,
            &[(from, fail)],
        )
        .unwrap_err();
        assert_eq!(error.code, UmweltErrorCode::MigrationFailed);
        let details = error.details.unwrap();
        assert_eq!(details["from"], from);
        assert!(PathBuf::from(details["backup"].as_str().unwrap()).exists());
        assert!(!has_table(&connection, "migrated"));
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, from);
    }

    #[test]
    fn a_missing_step_fails_without_changing_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = older_file(directory.path());
        let from = crate::FILE_VERSION - 1;
        let mut connection = open(&path);
        let error = migrate(&mut connection, &path, from, crate::FILE_VERSION, &[]).unwrap_err();
        assert_eq!(error.code, UmweltErrorCode::MigrationFailed);
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, from);
    }
}
