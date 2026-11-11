// SPDX-License-Identifier: AGPL-3.0-only
use rusqlite::Connection;
use tempfile::tempdir;
use umwelt_core::{RecordQuery, ScopeAncestor, UmweltErrorCode};
use umwelt_sqlite::{Durability, SqliteStore};

const KEY: &str = "a0a8d5482c1eb719682e2cf8ef931adb41673a7d6ce1c509a625e93cfa1ed391";

fn plan(store: &SqliteStore, query: RecordQuery) -> String {
    let details = store
        .explain_record_query(&query)
        .expect("explain record query");
    assert!(!details.is_empty(), "query produced no plan");
    details.join("\n")
}

/// Every paginated access path must (1) use its selective index with the
/// expected equality columns and (2) read rows in index order: a plan that
/// sorts ("TEMP B-TREE") would sort the holder's whole matching history
/// before applying LIMIT.
type AccessCase = (&'static str, QueryBuilder, &'static str, &'static str);

fn claim_access_cases() -> Vec<AccessCase> {
    vec![
        (
            "holdings without filter",
            query().holder("alice").kind("holding"),
            "claims_holder_kind_order",
            "holder_id=? AND kind=?",
        ),
        (
            "holder and subject",
            query().holder("alice").kind("holding").subject("subject"),
            "claims_holder_subject_order",
            "holder_id=? AND subject_id=?",
        ),
        (
            "holder and claim key",
            query().holder("alice").kind("holding").claim_key(KEY),
            "claims_holder_key_order",
            "holder_id=? AND claim_key=?",
        ),
        (
            "claim key without holder (who holds)",
            query().claim_key(KEY).kind("holding"),
            "claims_key_order",
            "claim_key=?",
        ),
        (
            "facts by claim key (contradictions)",
            query().claim_key(KEY).kind("fact"),
            "claims_key_order",
            "claim_key=?",
        ),
        (
            "facts by slot key (contradictions)",
            query().slot_key(KEY).kind("fact"),
            "claims_slot_kind",
            "slot_key=? AND kind=?",
        ),
        (
            "holder predicate and object",
            query()
                .holder("alice")
                .kind("holding")
                .predicate("related")
                .object("item"),
            "claims_holder_predicate_object",
            "holder_id=? AND predicate=? AND object_entity_id=?",
        ),
        (
            "fact subject and predicate",
            query().subject("subject").predicate("related").kind("fact"),
            "claims_fact_subject_predicate",
            "subject_id=? AND predicate=?",
        ),
        (
            "facts by object and predicate (group members)",
            query()
                .object("guild")
                .predicate("umwelt:member-of")
                .kind("fact"),
            "claims_fact_object",
            "object_entity_id=? AND predicate=?",
        ),
        (
            "canon without filter",
            query().kind("fact"),
            "claims_fact_order",
            "project_id=?",
        ),
    ]
}

#[test]
fn every_claim_access_path_uses_its_selective_index() {
    let store = SqliteStore::in_memory().expect("create store");
    for (name, query, expected_index, expected_columns) in claim_access_cases() {
        let plan = plan(&store, query.finish());
        assert!(!plan.contains("SCAN"), "{name}: {plan}");
        assert!(!plan.contains("TEMP B-TREE"), "{name}: {plan}");
        assert!(
            plan.contains(&format!("USING INDEX {expected_index}")),
            "{name}: {plan}"
        );
        assert!(plan.contains(expected_columns), "{name}: {plan}");
    }
}

/// `about` is two ordered index scans (subject branch and object branch). The
/// only sort allowed is the final merge, bounded by twice the page size.
#[test]
fn about_filter_uses_both_holder_indexes() {
    let store = SqliteStore::in_memory().expect("create store");
    let mut about = query().holder("alice").kind("holding").finish();
    about.about_entity_id = Some("mira".to_owned());
    let plan = plan(&store, about);
    assert!(!plan.contains("SCAN r"), "{plan}");
    assert!(
        plan.contains(
            "USING INDEX claims_holder_subject_order (project_id=? AND holder_id=? AND subject_id=?"
        ),
        "{plan}"
    );
    assert!(
        plan.contains("USING INDEX claims_holder_object_order (project_id=? AND holder_id=? AND object_entity_id=?"),
        "{plan}"
    );
}

#[test]
fn withheld_count_reads_only_the_partial_withheld_index() {
    let store = SqliteStore::in_memory().expect("create store");
    let mut about = query().holder("alice").kind("holding").finish();
    about.about_entity_id = Some("mira".to_owned());
    for query in [query().holder("alice").kind("holding").finish(), about] {
        let plan = store
            .explain_withheld_count(&query)
            .expect("explain withheld count")
            .join("\n");
        assert!(plan.contains("USING INDEX claims_withheld"), "{plan}");
        assert!(plan.contains("holder_id=?"), "{plan}");
        assert!(!plan.contains("SCAN"), "{plan}");
    }
}

#[test]
fn latest_holding_reads_one_row_from_the_holder_key_index() {
    let store = SqliteStore::in_memory().expect("create store");
    let plan = store
        .explain_latest_holding()
        .expect("explain latest holding")
        .join("\n");
    assert!(!plan.contains("TEMP B-TREE"), "{plan}");
    assert!(
        plan.contains(
            "USING INDEX claims_holder_key_order (project_id=? AND holder_id=? AND claim_key=?"
        ),
        "{plan}"
    );
    assert!(!plan.contains("SCAN"), "{plan}");
}

#[test]
fn invalid_claim_keys_match_nothing_instead_of_failing() {
    let store = SqliteStore::in_memory().expect("create store");
    assert!(
        store
            .explain_record_query(&query().claim_key("not-hex").finish())
            .expect("explain")
            .is_empty()
    );
}

#[test]
fn durability_defaults_to_normal_and_full_is_selectable() {
    let directory = tempdir().expect("create temporary directory");
    let normal = SqliteStore::create(directory.path().join("normal.umw")).expect("create");
    assert_eq!(normal.synchronous_level().expect("pragma"), 1);
    let full = SqliteStore::create_with(directory.path().join("full.umw"), Durability::Full)
        .expect("create full");
    assert_eq!(full.synchronous_level().expect("pragma"), 2);
    drop(full);
    let reopened = SqliteStore::open_with(directory.path().join("full.umw"), Durability::Normal)
        .expect("reopen");
    assert_eq!(reopened.synchronous_level().expect("pragma"), 1);
}

#[test]
fn current_window_lookup_uses_only_the_projects_primary_key() {
    let store = SqliteStore::in_memory().expect("create store");
    let plan = store
        .explain_current_window()
        .expect("explain current window")
        .join("\n");
    assert!(!plan.contains("SCAN"), "{plan}");
    assert!(plan.contains("USING INDEX sqlite_autoindex_projects_1 (id=?)"));
    assert!(!plan.contains("records"), "{plan}");
    assert!(!plan.contains("transitions"), "{plan}");
    assert!(!plan.contains("events"), "{plan}");
}

#[test]
fn file_lock_is_exclusive_and_released_on_drop() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("story.umw");
    let first = SqliteStore::create(&path).expect("create Umwelt file");
    let error = SqliteStore::open(&path).expect_err("second open must fail");
    assert_eq!(error.code, UmweltErrorCode::FileLocked);
    drop(first);
    SqliteStore::open(&path).expect("open after lock release");
}

#[test]
fn renamed_sqlite_database_is_not_an_umwelt_file() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("ordinary.umw");
    drop(Connection::open(&path).expect("create ordinary SQLite file"));
    let error = SqliteStore::open(&path).expect_err("ordinary database must be rejected");
    assert_eq!(error.code, UmweltErrorCode::NotUmweltFile);
}

#[test]
fn created_files_have_identity_and_other_versions_are_rejected() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("versioned.umw");
    drop(SqliteStore::create(&path).expect("create Umwelt file"));
    let connection = Connection::open(&path).expect("inspect Umwelt file");
    let application_id: i64 = connection
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .expect("read application id");
    let user_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user version");
    assert_eq!(application_id, 0x554D_5754);
    assert_eq!(user_version, 5);
    for other in [4, 6] {
        connection
            .pragma_update(None, "user_version", other)
            .expect("set another version");
        let error = SqliteStore::open(&path).expect_err("other versions must fail");
        assert_eq!(error.code, UmweltErrorCode::UnsupportedFileVersion);
    }
    drop(connection);
}

#[test]
fn opening_a_missing_file_does_not_create_database_or_lock_file() {
    let directory = tempdir().expect("create temporary directory");
    let path = directory.path().join("missing.umw");
    let lock_path = directory.path().join("missing.umw.lock");
    let error = SqliteStore::open(&path).expect_err("missing file must fail");
    assert_eq!(error.code, UmweltErrorCode::NotFound);
    assert!(!path.exists());
    assert!(!lock_path.exists());
}

/// Draft reads see their own rows plus their ancestors' up to each branch
/// point. That visibility is a residual filter: every path keeps its index and
/// its order, for a draft of main as for one nested four deep.
#[test]
fn draft_reads_use_the_same_indexes_as_main() {
    let store = SqliteStore::in_memory().expect("create store");
    let cases = claim_access_cases()
        .into_iter()
        .map(|(name, query, index, columns)| (name, query.in_draft().finish(), index, columns))
        .chain(
            claim_access_cases()
                .into_iter()
                .map(|(name, query, index, columns)| {
                    (name, query.in_nested_draft().finish(), index, columns)
                }),
        );
    for (name, query, expected_index, expected_columns) in cases {
        let plan = plan(&store, query);
        assert!(!plan.contains("SCAN r"), "{name}: {plan}");
        assert!(!plan.contains("TEMP B-TREE"), "{name}: {plan}");
        assert!(
            plan.contains(&format!("USING INDEX {expected_index}")),
            "{name}: {plan}"
        );
        assert!(plan.contains(expected_columns), "{name}: {plan}");
        assert!(
            plan.contains("transitions_target_window"),
            "draft reads check transitions: {name}: {plan}"
        );
        assert!(!plan.contains("current_status"), "{name}: {plan}");
    }
    let mut about = query().holder("alice").kind("holding").in_draft().finish();
    about.about_entity_id = Some("mira".to_owned());
    let about_plan = plan(&store, about.clone());
    assert!(
        about_plan.contains("USING INDEX claims_holder_subject_order"),
        "{about_plan}"
    );
    assert!(
        about_plan.contains("USING INDEX claims_holder_object_order"),
        "{about_plan}"
    );
    let withheld = store
        .explain_withheld_count(&about)
        .expect("explain withheld count")
        .join(
            "
",
        );
    assert!(
        withheld.contains("USING INDEX claims_withheld"),
        "{withheld}"
    );
    let latest = store
        .explain_latest_holding_in_draft()
        .expect("explain latest holding")
        .join(
            "
",
        );
    assert!(!latest.contains("TEMP B-TREE"), "{latest}");
    assert!(
        latest.contains(
            "USING INDEX claims_holder_key_order (project_id=? AND holder_id=? AND claim_key=?"
        ),
        "{latest}"
    );
}

/// Main reads one ordered range of the events index. A draft reads one per
/// timeline of its scope (its own and each ancestor's slot) and merges at most
/// five pages.
#[test]
fn event_pages_read_the_timeline_index() {
    let store = SqliteStore::in_memory().expect("create store");
    let main = store.explain_event_page(false).expect("explain").join(
        "
",
    );
    assert!(!main.contains("TEMP B-TREE"), "{main}");
    assert!(
        main.contains("USING INDEX events_timeline_order (project_id=? AND timeline_id=?"),
        "{main}"
    );
    let draft = store.explain_event_page(true).expect("explain").join(
        "
",
    );
    assert_eq!(
        draft
            .matches("USING INDEX events_timeline_order (project_id=? AND timeline_id=?")
            .count(),
        5,
        "{draft}"
    );
    assert!(!draft.contains("SCAN events"), "{draft}");
}

/// Promote and purge must find a draft's rows through an index; scanning the
/// whole ledger would make every night of a game cost the size of its history.
/// Only the full-text deletes scan, because FTS5 cannot index those columns.
#[test]
fn promote_and_purge_reach_draft_rows_through_indexes() {
    let store = SqliteStore::in_memory().expect("create store");
    for (sql, plan) in store.explain_draft_maintenance().expect("explain") {
        let plan = plan.join(
            "
",
        );
        let scans = plan
            .lines()
            .filter(|line| line.contains("SCAN") && !line.contains("search_fts"))
            .collect::<Vec<_>>();
        assert!(
            scans.is_empty(),
            "{sql}
{plan}"
        );
        assert!(
            plan.contains("USING"),
            "{sql}
{plan}"
        );
    }
}

/// Secrets, gossip sources and witnessed events are found through their own
/// indexes, never by scanning claims or records.
#[test]
fn knowledge_lookups_use_their_indexes() {
    let store = SqliteStore::in_memory().expect("create store");
    let plans = store.explain_knowledge_lookups().expect("explain");
    let expected = [
        ("secrets", "USING INDEX claims_secret (project_id=?"),
        (
            "secrets of holder",
            "USING INDEX claims_secret (project_id=? AND holder_id=?",
        ),
        (
            "told from",
            "USING INDEX acquisitions_source_record (project_id=? AND source_record_id=?",
        ),
        (
            "witnessed by",
            "USING INDEX acquisitions_witnessed (project_id=? AND acquisition_ref=?",
        ),
    ];
    for ((name, plan), (expected_name, expected_index)) in plans.iter().zip(expected) {
        assert_eq!(*name, expected_name);
        assert!(plan.contains(expected_index), "{name}: {plan}");
        assert!(!plan.contains("SCAN r"), "{name}: {plan}");
    }
}

/// A read on an additional axis replaces the tick bound with a primary-key
/// lookup in `event_positions` (for the record and for its transitions). Every
/// path keeps its index and its order, in main as in a nested draft, and takes
/// the historical path.
#[test]
fn axis_reads_use_the_same_indexes_as_main() {
    let store = SqliteStore::in_memory().expect("create store");
    let position_lookup = "SEARCH p USING PRIMARY KEY (project_id=? AND event_id=? AND axis_id=?)";
    let cases = claim_access_cases()
        .into_iter()
        .map(|(name, query, index, columns)| (name, query.on_axis().finish(), index, columns))
        .chain(
            claim_access_cases()
                .into_iter()
                .map(|(name, query, index, columns)| {
                    (
                        name,
                        query.on_axis().in_nested_draft().finish(),
                        index,
                        columns,
                    )
                }),
        );
    for (name, query, expected_index, expected_columns) in cases {
        let plan = plan(&store, query);
        assert!(!plan.contains("SCAN r"), "{name}: {plan}");
        assert!(!plan.contains("TEMP B-TREE"), "{name}: {plan}");
        assert!(
            plan.contains(&format!("USING INDEX {expected_index}")),
            "{name}: {plan}"
        );
        assert!(plan.contains(expected_columns), "{name}: {plan}");
        assert!(plan.contains("transitions_target_window"), "{name}: {plan}");
        assert_eq!(plan.matches(position_lookup).count(), 2, "{name}: {plan}");
        assert!(!plan.contains("current_status"), "{name}: {plan}");
    }
    let withheld = store
        .explain_withheld_count(&query().holder("alice").kind("holding").on_axis().finish())
        .expect("explain withheld count")
        .join("\n");
    assert!(
        withheld.contains("USING INDEX claims_withheld"),
        "{withheld}"
    );
    assert!(withheld.contains(position_lookup), "{withheld}");
    let latest = store
        .explain_latest_holding_on_axis()
        .expect("explain latest holding")
        .join("\n");
    assert!(!latest.contains("TEMP B-TREE"), "{latest}");
    assert!(
        latest.contains(
            "USING INDEX claims_holder_key_order (project_id=? AND holder_id=? AND claim_key=?"
        ),
        "{latest}"
    );
    assert!(latest.contains(position_lookup), "{latest}");
}

/// An event page on an additional axis is one ordered range of
/// `event_positions_order`, each event reached by its primary key.
#[test]
fn event_pages_on_an_axis_read_the_position_index() {
    let store = SqliteStore::in_memory().expect("create store");
    let plan = store
        .explain_event_page_on_axis()
        .expect("explain")
        .join("\n");
    assert!(!plan.contains("TEMP B-TREE"), "{plan}");
    assert!(
        plan.contains("USING COVERING INDEX event_positions_order (project_id=? AND axis_id=?")
            || plan.contains("USING INDEX event_positions_order (project_id=? AND axis_id=?"),
        "{plan}"
    );
    assert!(
        plan.contains("SEARCH e USING PRIMARY KEY (project_id=? AND id=?)"),
        "{plan}"
    );
    assert!(!plan.contains("SCAN"), "{plan}");
}

/// A semantic search filters in SQL before it scores: with a holder it walks
/// the holder's Holdings and reaches each vector by its key; without one it
/// reads one space's vectors by key prefix and reaches each document by its
/// key. Neither scans the ledger.
#[test]
fn semantic_search_reaches_vectors_and_documents_by_key() {
    let store = SqliteStore::in_memory().expect("create store");
    let holder = store.explain_semantic_search(true).expect("explain").join(
        "
",
    );
    assert!(
        holder.contains(
            "USING INDEX claims_holder_kind_order (project_id=? AND holder_id=? AND kind=?"
        ),
        "{holder}"
    );
    assert!(
        holder.contains(
            "SEARCH m USING INDEX embeddings_doc (project_id=? AND space_id=? AND doc_type=? AND doc_id=?)"
        ),
        "{holder}"
    );
    assert!(!holder.contains("SCAN"), "{holder}");
    let narrator = store.explain_semantic_search(false).expect("explain").join(
        "
",
    );
    assert!(
        narrator.contains("SEARCH m USING INDEX embeddings_doc (project_id=? AND space_id=?"),
        "{narrator}"
    );
    assert!(
        narrator.contains("SEARCH r USING PRIMARY KEY (project_id=? AND id=?)"),
        "{narrator}"
    );
    assert!(
        narrator.contains("SEARCH e USING PRIMARY KEY (project_id=? AND id=?)"),
        "{narrator}"
    );
    assert!(!narrator.contains("SCAN"), "{narrator}");
}

/// `embedding.pending` walks the full-text index in rowid order from the
/// cursor and reaches documents and vectors by their keys.
#[test]
fn pending_embeddings_reach_documents_and_vectors_by_key() {
    let store = SqliteStore::in_memory().expect("create store");
    let plan = store.explain_pending_embeddings().expect("explain").join(
        "
",
    );
    assert!(plan.contains("SCAN f VIRTUAL TABLE"), "{plan}");
    assert!(
        plan.contains("SEARCH r USING PRIMARY KEY (project_id=? AND id=?)"),
        "{plan}"
    );
    assert!(
        plan.contains("SEARCH e USING PRIMARY KEY (project_id=? AND id=?)"),
        "{plan}"
    );
    assert!(
        plan.contains(
            "SEARCH m USING INDEX embeddings_doc (project_id=? AND space_id=? AND doc_type=? AND doc_id=?)"
        ),
        "{plan}"
    );
    assert!(!plan.contains("TEMP B-TREE"), "{plan}");
}

struct QueryBuilder(RecordQuery);

fn query() -> QueryBuilder {
    QueryBuilder(RecordQuery {
        axis: None,
        project_id: "project".to_owned(),
        timeline_id: "main".to_owned(),
        ancestors: Vec::new(),
        holder_id: None,
        record_kind: None,
        claim_key: None,
        slot_key: None,
        subject_id: None,
        predicate: None,
        object_entity_id: None,
        about_entity_id: None,
        attitudes: Vec::new(),
        minimum_confidence: None,
        include_withheld: false,
        tick: i64::MAX,
        as_of_version: i64::MAX as u64,
        limit: 50,
        cursor: None,
    })
}

impl QueryBuilder {
    fn holder(mut self, value: &str) -> Self {
        self.0.holder_id = Some(value.to_owned());
        self
    }

    fn kind(mut self, value: &str) -> Self {
        self.0.record_kind = Some(value.to_owned());
        self
    }

    fn claim_key(mut self, value: &str) -> Self {
        self.0.claim_key = Some(value.to_owned());
        self
    }

    fn slot_key(mut self, value: &str) -> Self {
        self.0.slot_key = Some(value.to_owned());
        self
    }

    fn subject(mut self, value: &str) -> Self {
        self.0.subject_id = Some(value.to_owned());
        self
    }

    fn predicate(mut self, value: &str) -> Self {
        self.0.predicate = Some(value.to_owned());
        self
    }

    fn object(mut self, value: &str) -> Self {
        self.0.object_entity_id = Some(value.to_owned());
        self
    }

    fn on_axis(mut self) -> Self {
        self.0.axis = Some("reading".to_owned());
        self
    }

    fn in_draft(mut self) -> Self {
        self.0.timeline_id = "draft".to_owned();
        self.0.ancestors = vec![ScopeAncestor::new("main", 5)];
        self
    }

    /// A draft nested as deep as drafts go: three draft ancestors and main.
    fn in_nested_draft(mut self) -> Self {
        self.0.timeline_id = "draft-4".to_owned();
        self.0.ancestors = vec![
            ScopeAncestor::new("draft-3", 9),
            ScopeAncestor::new("draft-2", 8),
            ScopeAncestor::new("draft-1", 7),
            ScopeAncestor::new("main", 5),
        ];
        self
    }

    fn finish(self) -> RecordQuery {
        self.0
    }
}

/// The `dispositions` read starts from the holder's records, on the main
/// axis and on an additional one, and never scans the ledger.
#[test]
fn dispositions_start_from_the_holder() {
    let store = SqliteStore::in_memory().expect("create store");
    for axis in [false, true] {
        let plan = store
            .explain_dispositions(axis)
            .expect("explain")
            .join("\n");
        assert!(
            plan.contains(
                "USING INDEX claims_holder_kind_order (project_id=? AND holder_id=? AND kind=?"
            ),
            "{plan}"
        );
        assert!(
            plan.contains("SEARCH r USING PRIMARY KEY (project_id=? AND id=?)"),
            "{plan}"
        );
        // Only the ranked rows are walked, never the records or transitions.
        assert!(
            !plan.lines().any(|line| {
                ["SCAN r", "SCAN t"]
                    .iter()
                    .any(|scan| line == *scan || line.starts_with(&format!("{scan} ")))
            }),
            "{plan}"
        );
    }
}
