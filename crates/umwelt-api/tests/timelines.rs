// SPDX-License-Identifier: AGPL-3.0-only
//! Draft timelines: drafts branch from main or from other drafts, are
//! promoted atomically into their parent or discarded.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;
use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore};
use umwelt_core::domain::*;
use umwelt_core::{UmweltError, UmweltErrorCode};

type TestEngine = Engine<SqliteStore>;

fn engine() -> TestEngine {
    let mut engine = Engine::new(
        SqliteStore::in_memory().expect("create store"),
        Arc::new(FixedClock("2026-10-07T12:00:00Z".to_owned())),
        Arc::new(SeqIdGen::new()),
    );
    engine
        .create_project(CreateProjectInput {
            id: Some("p".to_owned()),
            name: "Timelines".to_owned(),
            description: None,
            timescale: "day".to_owned(),
            vocabulary_mode: VocabularyMode::Strict,
        })
        .expect("create project");
    for (id, kind) in [("agent", EntityKind::Agent), ("other", EntityKind::Other)] {
        engine
            .create_entity_type(CreateEntityTypeInput {
                project_id: "p".to_owned(),
                id: Some(id.to_owned()),
                name: id.to_owned(),
                kind,
                fields: Vec::new(),
            })
            .expect("create entity type");
    }
    for (id, type_id) in [
        ("alice", "agent"),
        ("bob", "agent"),
        ("carol", "agent"),
        ("item", "other"),
    ] {
        engine
            .create_entity(CreateEntityInput {
                project_id: "p".to_owned(),
                id: Some(id.to_owned()),
                type_id: type_id.to_owned(),
                name: id.to_owned(),
                aliases: Vec::new(),
                fields: serde_json::Map::new(),
                external_ref: None,
                created_at: Position { tick: 0 },
            })
            .expect("create entity");
    }
    engine
        .define_predicate(DefinePredicateInput {
            project_id: "p".to_owned(),
            predicate: PredicateDef {
                id: "related".to_owned(),
                label: "Related".to_owned(),
                object: ObjectKind::Entity,
                cardinality: Cardinality::Many,
                subject_types: None,
            },
        })
        .expect("define predicate");
    engine
}

fn provenance() -> Provenance {
    Provenance {
        origin: Origin::Human { actor_id: None },
        reason: None,
        metadata: None,
    }
}

fn claim(subject: &str) -> Claim {
    Claim {
        subject: subject.to_owned(),
        predicate: "related".to_owned(),
        object: ClaimObject::Entity {
            id: "item".to_owned(),
        },
        qualifiers: BTreeMap::new(),
        polarity: true,
        about: None,
    }
}

fn holding(id: &str, holder: &str, tick: i64, text: Option<&str>) -> RecordInput {
    RecordInput::Holding {
        common: RecordInputCommon {
            id: Some(id.to_owned()),
            position: Some(Position { tick }),
            provenance: Some(provenance()),
        },
        holder: holder.to_owned(),
        attitude: Attitude::Believes,
        claim: claim(holder),
        acquisition: Acquisition::Authored,
        confidence: 1.0,
        accessibility: Accessibility::Available,
        text: text.map(str::to_owned),
        secrecy: None,
    }
}

fn fact(id: &str, subject: &str, tick: i64) -> RecordInput {
    RecordInput::Fact {
        common: RecordInputCommon {
            id: Some(id.to_owned()),
            position: Some(Position { tick }),
            provenance: Some(provenance()),
        },
        claim: claim(subject),
    }
}

fn forget(id: &str, target: &str, tick: i64) -> TransitionInput {
    TransitionInput {
        id: Some(id.to_owned()),
        target_record_id: target.to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick }),
        provenance: Some(provenance()),
    }
}

/// A commit with a fixed event id (`e-<attempt>`), so replays and re-sent days
/// carry the same ids.
fn commit(
    timeline: Option<&str>,
    attempt: &str,
    tick: i64,
    title: &str,
    records: Vec<RecordInput>,
    transitions: Vec<TransitionInput>,
) -> CommitInput {
    CommitInput {
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: timeline.map(str::to_owned),
        expected_version: None,
        event: EventInput {
            on: Default::default(),
            id: Some(format!("e-{attempt}")),
            event_type: "scene".to_owned(),
            title: title.to_owned(),
            description: None,
            position: Position { tick },
            participants: Vec::new(),
            location: None,
            evidence: Vec::new(),
            provenance: provenance(),
        },
        records,
        transitions,
        preconditions: Vec::new(),
    }
}

fn open(engine: &mut TestEngine, id: &str) -> Timeline {
    engine
        .open_draft(OpenDraftInput {
            project_id: "p".to_owned(),
            id: Some(id.to_owned()),
            label: None,
            base_timeline_id: None,
        })
        .expect("open draft")
}

fn open_from(engine: &mut TestEngine, id: &str, base: &str) -> Result<Timeline, UmweltError> {
    engine.open_draft(OpenDraftInput {
        project_id: "p".to_owned(),
        id: Some(id.to_owned()),
        label: None,
        base_timeline_id: Some(base.to_owned()),
    })
}

fn holds(engine: &TestEngine, timeline: &str, holder: &str) -> bool {
    engine
        .in_timeline(timeline)
        .holds(
            "p",
            holder,
            ClaimRef::Claim {
                claim: claim(holder),
            },
            At::default(),
        )
        .expect("holds")
        .holds
}

fn holding_ids(engine: &TestEngine, timeline: &str, holder: &str) -> Vec<String> {
    engine
        .in_timeline(timeline)
        .holdings(
            "p",
            holder,
            HoldingFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("holdings")
        .items
        .into_iter()
        .map(|item| record_id(&item.record))
        .collect()
}

fn record_id(record: &Record) -> String {
    match record {
        Record::Fact { common, .. }
        | Record::Holding { common, .. }
        | Record::Awareness { common, .. }
        | Record::Disposition { common, .. } => common.id.clone(),
    }
}

fn assert_code(error: UmweltError, code: UmweltErrorCode) -> UmweltError {
    assert_eq!(error.code, code, "{}", error.message);
    error
}

#[test]
fn draft_sees_main_up_to_its_base_and_main_sees_nothing_of_the_draft() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Before",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("main before draft");
    let draft = open(&mut engine, "d");
    assert_eq!(draft.base_version, Some(1));
    engine
        .commit(commit(
            None,
            "m2",
            2,
            "After",
            vec![fact("f2", "alice", 2)],
            vec![],
        ))
        .expect("main after draft");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            2,
            "Draft",
            vec![holding("h3", "bob", 2, None)],
            vec![],
        ))
        .expect("draft commit");

    assert_eq!(holding_ids(&engine, "d", "alice"), ["h1"]);
    assert_eq!(holding_ids(&engine, "d", "bob"), ["h3"]);
    assert_eq!(holding_ids(&engine, "main", "bob"), Vec::<String>::new());
    let draft_canon = engine
        .in_timeline("d")
        .canon("p", CanonFilter::default(), At::default(), Page::default())
        .expect("draft canon");
    assert!(
        draft_canon.items.is_empty(),
        "main rows after the base leak into the draft"
    );
    assert_code(
        engine
            .record("p", "h3")
            .expect_err("main must not see draft records"),
        UmweltErrorCode::NotFound,
    );
    assert_code(
        engine
            .in_timeline("d")
            .record("p", "f2")
            .expect_err("draft must not see main rows after its base"),
        UmweltErrorCode::NotFound,
    );
    let events = engine
        .in_timeline("d")
        .timeline("p", At::default(), Page::default())
        .expect("draft events");
    let titles = events
        .items
        .iter()
        .map(|event| event.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(titles, ["Before", "Draft"]);
}

#[test]
fn promote_applies_the_whole_draft_to_main_at_once() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Before",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("main commit");
    open(&mut engine, "d");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            2,
            "Day",
            vec![holding("h2", "bob", 2, None)],
            vec![],
        ))
        .expect("draft commit 1");
    engine
        .commit(commit(
            Some("d"),
            "d2",
            3,
            "Night",
            vec![fact("f3", "carol", 3)],
            vec![],
        ))
        .expect("draft commit 2");
    assert!(!holds(&engine, "main", "bob"));

    let promoted = engine.promote_draft("p", "d").expect("promote");
    assert_eq!(promoted.status, TimelineStatus::Promoted);
    assert!(promoted.closed_at.is_some());
    let main = engine.get_timeline("p", "main").expect("main");
    assert_eq!(
        engine.project("p").expect("project").version,
        main.head_version
    );
    assert_eq!(main.head_version, promoted.head_version);
    assert_eq!(main.max_tick, 3);
    assert!(holds(&engine, "main", "bob"));
    let record = engine.record("p", "h2").expect("promoted record");
    assert!(
        matches!(record, Record::Holding { common, .. } if common.timeline_id == MAIN_TIMELINE_ID)
    );
    let canon = engine
        .canon("p", CanonFilter::default(), At::default(), Page::default())
        .expect("canon");
    assert_eq!(canon.items.len(), 1);
    let feed = engine.changes("p", 0, 100).expect("changes").items;
    assert!(feed.iter().all(|item| item.timeline_id == MAIN_TIMELINE_ID));
    let last = feed.last().expect("promotion is in the feed");
    assert_eq!(last.change_type, "timeline.promoted");
    assert_eq!(last.payload["timelineId"], "d");
}

#[test]
fn promote_after_main_moved_is_a_version_conflict_and_keeps_the_draft() {
    let mut engine = engine();
    open(&mut engine, "d");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            1,
            "Day",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("draft commit");
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Elsewhere",
            vec![holding("h2", "bob", 1, None)],
            vec![],
        ))
        .expect("main moves on");

    let error = assert_code(
        engine.promote_draft("p", "d").expect_err("main moved"),
        UmweltErrorCode::VersionConflict,
    );
    let details = error.details.expect("conflict details");
    assert_eq!(details["baseVersion"], 0);
    assert_eq!(details["mainHeadVersion"], 2);
    let draft = engine.get_timeline("p", "d").expect("draft");
    assert_eq!(draft.status, TimelineStatus::Active);
    assert!(holds(&engine, "d", "alice"));
    assert!(!holds(&engine, "main", "alice"));
    engine
        .commit(commit(
            Some("d"),
            "d2",
            2,
            "Still open",
            vec![fact("f1", "alice", 2)],
            vec![],
        ))
        .expect("the draft still accepts writes");
}

#[test]
fn of_two_drafts_from_the_same_base_only_the_first_promotes() {
    let mut engine = engine();
    open(&mut engine, "d1");
    open(&mut engine, "d2");
    engine
        .commit(commit(
            Some("d1"),
            "a1",
            1,
            "One",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("draft one");
    engine
        .commit(commit(
            Some("d2"),
            "a2",
            1,
            "Two",
            vec![holding("h2", "bob", 1, None)],
            vec![],
        ))
        .expect("draft two");
    engine.promote_draft("p", "d1").expect("first promote");
    assert_code(
        engine.promote_draft("p", "d2").expect_err("second promote"),
        UmweltErrorCode::VersionConflict,
    );
    assert!(holds(&engine, "main", "alice"));
    assert!(!holds(&engine, "main", "bob"));
}

#[test]
fn discard_archives_the_draft_readable_only_by_its_id() {
    let mut engine = engine();
    open(&mut engine, "d");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            1,
            "Day",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("draft commit");
    let discarded = engine.discard_draft("p", "d", false).expect("discard");
    assert_eq!(discarded.status, TimelineStatus::Discarded);
    assert!(discarded.purged_at.is_none());

    assert!(!holds(&engine, "main", "alice"));
    assert!(
        engine
            .who_holds(
                "p",
                ClaimRef::Claim {
                    claim: claim("alice")
                },
                At::default(),
                WhoHoldsFilter::default(),
            )
            .expect("who holds")
            .is_empty()
    );
    assert!(holds(&engine, "d", "alice"));
    assert!(engine.in_timeline("d").record("p", "h1").is_ok());
    let error = assert_code(
        engine
            .commit(commit(
                Some("d"),
                "d2",
                2,
                "Late",
                vec![fact("f1", "alice", 2)],
                vec![],
            ))
            .expect_err("discarded drafts accept no writes"),
        UmweltErrorCode::InvalidState,
    );
    assert_eq!(error.details.expect("details")["status"], "discarded");
    let active = engine
        .timelines("p", Some(TimelineStatus::Active))
        .expect("active timelines");
    assert_eq!(
        active
            .iter()
            .map(|value| value.id.as_str())
            .collect::<Vec<_>>(),
        ["main"]
    );
}

#[test]
fn purge_removes_the_rows_and_frees_ids_and_attempts() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Main",
            vec![holding("h0", "carol", 1, None)],
            vec![],
        ))
        .expect("main commit");
    let before = engine.counts("p").expect("counts before");
    open(&mut engine, "d");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            1,
            "Day",
            vec![holding("h1", "alice", 1, Some("lantern"))],
            vec![forget("t1", "h0", 2)],
        ))
        .expect("draft commit");
    engine.discard_draft("p", "d", false).expect("discard");

    let result = engine.purge_draft("p", "d").expect("purge");
    assert!(result.purged);
    assert_eq!(
        result.removed,
        PurgeCounts {
            events: 1,
            records: 1,
            transitions: 1,
            attempts: 1,
        }
    );
    assert_eq!(engine.counts("p").expect("counts after"), before);
    assert!(
        engine
            .get_timeline("p", "d")
            .expect("timeline row stays")
            .purged_at
            .is_some()
    );
    let again = engine
        .purge_draft("p", "d")
        .expect("purging twice is a no-op");
    assert_eq!(again.removed.records, 0);

    let outcome = engine
        .commit(commit(
            None,
            "d1",
            1,
            "Day",
            vec![holding("h1", "alice", 1, Some("lantern"))],
            vec![forget("t1", "h0", 2)],
        ))
        .expect("ids and attempt are free again");
    assert_eq!(outcome.event_id, "e-d1");
    assert!(holds(&engine, "main", "alice"));
    assert!(!holds(&engine, "main", "carol"));
}

/// Ids are unique per project across timelines. A draft that is discarded but
/// not purged keeps its ids reserved.
#[test]
fn discarded_but_unpurged_draft_keeps_its_ids_reserved() {
    let mut engine = engine();
    open(&mut engine, "d1");
    engine
        .commit(commit(
            Some("d1"),
            "a1",
            1,
            "Day",
            vec![holding("hx", "alice", 1, None)],
            vec![],
        ))
        .expect("first draft");
    engine.discard_draft("p", "d1", false).expect("discard");
    open(&mut engine, "d2");
    let error = assert_code(
        engine
            .commit(commit(
                Some("d2"),
                "a2",
                1,
                "Again",
                vec![holding("hx", "alice", 1, None)],
                vec![],
            ))
            .expect_err("id still reserved"),
        UmweltErrorCode::Conflict,
    );
    assert!(error.message.contains("hx"));
}

#[test]
fn a_draft_transition_on_a_main_record_only_affects_the_draft_until_promoted() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Learn",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("main holding");
    open(&mut engine, "d");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            2,
            "Forget",
            vec![],
            vec![forget("t1", "h1", 2)],
        ))
        .expect("draft transition");

    assert!(holds(&engine, "main", "alice"));
    assert!(!holds(&engine, "d", "alice"));
    let trace = engine.trace("p", "h1").expect("main trace");
    assert!(trace.transitions.is_empty());
    let draft_trace = engine
        .in_timeline("d")
        .trace("p", "h1")
        .expect("draft trace");
    assert_eq!(draft_trace.status, Some(TransitionStatus::Forgotten));

    engine.promote_draft("p", "d").expect("promote");
    assert!(!holds(&engine, "main", "alice"));
    assert_eq!(
        engine.trace("p", "h1").expect("trace").status,
        Some(TransitionStatus::Forgotten)
    );
}

#[test]
fn transmit_in_a_draft_reads_main_sources_and_replays_idempotently() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Learn",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("main source");
    open(&mut engine, "d");
    let transmit = TransmitInput {
        on: Default::default(),
        project_id: "p".to_owned(),
        attempt_id: "tx".to_owned(),
        timeline_id: Some("d".to_owned()),
        position: Position { tick: 2 },
        from: "alice".to_owned(),
        to: "bob".to_owned(),
        source_record_id: "h1".to_owned(),
        record_id: Some("bob-heard".to_owned()),
        attitude: Attitude::Believes,
        confidence: 0.8,
        intent: Intent::Honest,
        distorted_claim: None,
        text: None,
    };
    let first = engine
        .transmit(transmit.clone())
        .expect("transmit in draft");
    let counts = engine.counts("p").expect("counts");
    let replay = engine.transmit(transmit).expect("replay");
    assert_eq!(first, replay);
    assert_eq!(engine.counts("p").expect("counts after replay"), counts);

    let heard = engine
        .in_timeline("d")
        .record("p", "bob-heard")
        .expect("draft record");
    assert!(matches!(heard, Record::Holding { claim: copied, .. } if copied == claim("alice")));
    assert!(!holds(&engine, "main", "bob"));
}

#[test]
fn expected_version_is_checked_against_the_target_timeline_head() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "One",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("v1");
    open(&mut engine, "d");
    engine
        .commit(commit(
            None,
            "m2",
            1,
            "Two",
            vec![holding("h2", "bob", 1, None)],
            vec![],
        ))
        .expect("v2 in main");

    let mut in_draft = commit(
        Some("d"),
        "d1",
        1,
        "Draft",
        vec![fact("f1", "alice", 1)],
        vec![],
    );
    in_draft.expected_version = Some(1);
    let outcome = engine.commit(in_draft).expect("draft head is still 1");
    assert_eq!(outcome.version, 3);

    let mut stale = commit(
        Some("d"),
        "d2",
        1,
        "Stale",
        vec![fact("f2", "bob", 1)],
        vec![],
    );
    stale.expected_version = Some(1);
    let error = assert_code(
        engine.commit(stale).expect_err("draft moved to 3"),
        UmweltErrorCode::VersionConflict,
    );
    assert_eq!(error.details.expect("details")["headVersion"], 3);

    let mut main_stale = commit(None, "m3", 1, "Main", vec![fact("f3", "carol", 1)], vec![]);
    main_stale.expected_version = Some(3);
    assert_code(
        engine.commit(main_stale.clone()).expect_err("main is at 2"),
        UmweltErrorCode::VersionConflict,
    );
    main_stale.expected_version = Some(2);
    engine.commit(main_stale).expect("main head is 2");
}

#[test]
fn search_in_a_draft_sees_its_text_and_main_up_to_the_base() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Alpha festival",
            vec![holding("h1", "alice", 1, Some("alpha"))],
            vec![],
        ))
        .expect("main before");
    open(&mut engine, "d");
    engine
        .commit(commit(
            None,
            "m2",
            2,
            "Gamma festival",
            vec![fact("f2", "bob", 2)],
            vec![],
        ))
        .expect("main after");
    engine
        .commit(commit(
            Some("d"),
            "d1",
            2,
            "Beta festival",
            vec![holding("h3", "alice", 2, Some("beta"))],
            vec![],
        ))
        .expect("draft");
    let search = |timeline: &str, text: &str, holder: Option<&str>| {
        engine
            .in_timeline(timeline)
            .search(
                "p",
                text,
                SearchFilter {
                    holder: holder.map(str::to_owned),
                    doc_type: None,
                },
                At::default(),
                Page::default(),
            )
            .expect("search")
            .items
            .into_iter()
            .map(|item| item.doc_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(search("d", "beta", Some("alice")), ["h3"]);
    assert_eq!(search("d", "alpha", Some("alice")), ["h1"]);
    let mut draft_events = search("d", "festival", None);
    draft_events.sort();
    assert_eq!(draft_events, ["e-d1", "e-m1"]);
    let mut main_events = search("main", "festival", None);
    main_events.sort();
    assert_eq!(main_events, ["e-m1", "e-m2"]);
    assert!(search("main", "beta", Some("alice")).is_empty());
}

#[test]
fn draft_ids_and_bases_are_validated() {
    let mut engine = engine();
    open(&mut engine, "d");
    assert_code(
        open_from(&mut engine, "main", "d").expect_err("main is not a draft id"),
        UmweltErrorCode::ValidationFailed,
    );
    assert_code(
        open_from(&mut engine, "d", "main").expect_err("duplicate draft id"),
        UmweltErrorCode::Conflict,
    );
    assert_code(
        open_from(&mut engine, "orphan", "missing").expect_err("unknown base"),
        UmweltErrorCode::NotFound,
    );

    open_from(&mut engine, "closed", "main").expect("open");
    engine.discard_draft("p", "closed", false).expect("discard");
    assert_code(
        open_from(&mut engine, "from-discarded", "closed").expect_err("discarded base"),
        UmweltErrorCode::InvalidState,
    );
    engine.purge_draft("p", "closed").expect("purge");
    assert_code(
        open_from(&mut engine, "from-purged", "closed").expect_err("purged base"),
        UmweltErrorCode::NotFound,
    );
    open_from(&mut engine, "done", "main").expect("open");
    engine.promote_draft("p", "done").expect("promote");
    assert_code(
        open_from(&mut engine, "from-promoted", "done").expect_err("promoted base"),
        UmweltErrorCode::InvalidState,
    );
}

/// Drafts nest at most `MAX_DRAFT_DEPTH` deep below main.
#[test]
fn drafts_nest_up_to_the_maximum_depth() {
    let mut engine = engine();
    let mut parent = MAIN_TIMELINE_ID.to_owned();
    for depth in 1..=MAX_DRAFT_DEPTH {
        let id = format!("d{depth}");
        let draft = open_from(&mut engine, &id, &parent).expect("open nested draft");
        assert_eq!(draft.depth, depth);
        assert_eq!(draft.parent_timeline_id.as_deref(), Some(parent.as_str()));
        parent = id;
    }
    let error = assert_code(
        open_from(&mut engine, "too-deep", &parent).expect_err("too deep"),
        UmweltErrorCode::ValidationFailed,
    );
    let details = error.details.expect("details");
    assert_eq!(details["maxDepth"], MAX_DRAFT_DEPTH);
    assert_eq!(details["parentTimelineId"], parent.as_str());
    let main = engine.get_timeline("p", MAIN_TIMELINE_ID).expect("main");
    assert_eq!((main.depth, main.parent_timeline_id), (0, None));
}

/// A nested draft sees its parent up to its own base and each further
/// ancestor up to the base of the draft below it; nothing it writes leaves it.
#[test]
fn nested_draft_sees_each_ancestor_up_to_its_branch_point() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Before",
            vec![holding("h-main", "alice", 1, None)],
            vec![],
        ))
        .expect("main before d1");
    open(&mut engine, "d1");
    engine
        .commit(commit(
            None,
            "m2",
            1,
            "Main later",
            vec![holding("h-main-late", "carol", 1, None)],
            vec![],
        ))
        .expect("main after d1");
    let d1_commit = engine
        .commit(commit(
            Some("d1"),
            "d1a",
            1,
            "Day",
            vec![holding("h-d1", "bob", 1, None)],
            vec![],
        ))
        .expect("d1 before d2");
    let d2 = open_from(&mut engine, "d2", "d1").expect("open d2");
    assert_eq!(d2.base_version, Some(d1_commit.version));
    assert_eq!(d2.depth, 2);
    engine
        .commit(commit(
            Some("d1"),
            "d1b",
            2,
            "Day later",
            vec![fact("f-d1-late", "bob", 2)],
            vec![],
        ))
        .expect("d1 after d2");
    engine
        .commit(commit(
            Some("d2"),
            "d2a",
            2,
            "Scene",
            vec![holding("h-d2", "carol", 2, None)],
            vec![],
        ))
        .expect("d2 commit");

    assert_eq!(holding_ids(&engine, "d2", "alice"), ["h-main"]);
    assert_eq!(holding_ids(&engine, "d2", "bob"), ["h-d1"]);
    assert_eq!(
        holding_ids(&engine, "d2", "carol"),
        ["h-d2"],
        "main after d1 opened is invisible"
    );
    assert!(
        engine.in_timeline("d2").record("p", "f-d1-late").is_err(),
        "d1 after d2 opened"
    );
    assert!(
        holding_ids(&engine, "d1", "carol").is_empty(),
        "d1 never sees d2"
    );
    assert_eq!(holding_ids(&engine, "main", "bob"), Vec::<String>::new());
    let events = engine
        .in_timeline("d2")
        .timeline("p", At::default(), Page::default())
        .expect("events")
        .items
        .into_iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    assert_eq!(events, ["e-m1", "e-d1a", "e-d2a"]);
}

#[test]
fn promote_moves_a_nested_draft_into_its_parent_then_main() {
    let mut engine = engine();
    open(&mut engine, "d1");
    open_from(&mut engine, "d2", "d1").expect("open d2");
    let scene = engine
        .commit(commit(
            Some("d2"),
            "d2a",
            1,
            "Scene",
            vec![holding("h-d2", "alice", 1, None)],
            vec![],
        ))
        .expect("d2 commit");
    let promoted = engine.promote_draft("p", "d2").expect("promote d2");
    assert_eq!(promoted.status, TimelineStatus::Promoted);
    assert!(holds(&engine, "d1", "alice"));
    assert!(
        !holds(&engine, "main", "alice"),
        "promoting into a draft confirms nothing"
    );
    let d1 = engine.get_timeline("p", "d1").expect("d1");
    assert_eq!(d1.head_version, scene.version);
    let error = assert_code(
        engine
            .in_timeline("d2")
            .record("p", "h-d2")
            .expect_err("promoted drafts are read through their parent"),
        UmweltErrorCode::InvalidState,
    );
    assert_eq!(error.details.expect("details")["promotedInto"], "d1");
    let promoted_item = engine
        .changes_in("p", 0, 100, Some("d1"))
        .expect("changes")
        .items
        .into_iter()
        .find(|item| item.change_type == "timeline.promoted")
        .expect("promoted item");
    assert_eq!(promoted_item.payload["into"], "d1");
    assert_eq!(promoted_item.payload["timelineId"], "d2");

    engine.promote_draft("p", "d1").expect("promote d1");
    assert!(holds(&engine, "main", "alice"));
    assert_eq!(engine.project("p").expect("project").version, scene.version);
    assert_eq!(
        engine
            .in_timeline("main")
            .record("p", "h-d2")
            .map(|record| record_id(&record))
            .expect("in main"),
        "h-d2"
    );
}

#[test]
fn promote_into_a_parent_that_moved_conflicts_and_rebase_fixes_it() {
    let mut engine = engine();
    open(&mut engine, "d1");
    open_from(&mut engine, "d2", "d1").expect("open d2");
    engine
        .commit(commit(
            Some("d2"),
            "d2a",
            1,
            "Scene",
            vec![holding("h-d2", "alice", 1, None)],
            vec![],
        ))
        .expect("d2 commit");
    let moved = engine
        .commit(commit(
            Some("d1"),
            "d1a",
            1,
            "Day",
            vec![holding("h-d1", "bob", 1, None)],
            vec![],
        ))
        .expect("d1 moves");
    let error = assert_code(
        engine.promote_draft("p", "d2").expect_err("parent moved"),
        UmweltErrorCode::VersionConflict,
    );
    let details = error.details.expect("details");
    assert_eq!(details["parentTimelineId"], "d1");
    assert_eq!(details["parentHeadVersion"], moved.version);
    assert!(details.get("mainHeadVersion").is_none());
    assert_eq!(
        engine.get_timeline("p", "d2").expect("d2").status,
        TimelineStatus::Active
    );

    let rebased = engine.rebase_draft("p", "d2").expect("rebase onto d1");
    assert_eq!(rebased.base_version, Some(moved.version));
    assert_eq!(
        holding_ids(&engine, "d2", "bob"),
        ["h-d1"],
        "d2 now sees d1's commit"
    );
    engine
        .promote_draft("p", "d2")
        .expect("promote after rebase");
    assert!(holds(&engine, "d1", "alice"));
}

/// Promoting a draft that has children hands them to its parent with their
/// own bases: every child reads exactly what it read before.
#[test]
fn promoting_a_middle_draft_hands_its_children_to_the_parent_unchanged() {
    let mut engine = engine();
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Before",
            vec![holding("h-main", "alice", 1, None)],
            vec![],
        ))
        .expect("main");
    open(&mut engine, "d1");
    engine
        .commit(commit(
            Some("d1"),
            "d1a",
            1,
            "Arc",
            vec![holding("h-d1", "bob", 1, None)],
            vec![],
        ))
        .expect("d1");
    open_from(&mut engine, "d2", "d1").expect("open d2");
    engine
        .commit(commit(
            Some("d2"),
            "d2a",
            2,
            "Chapter",
            vec![holding("h-d2", "carol", 2, None)],
            vec![],
        ))
        .expect("d2 before d3");
    open_from(&mut engine, "d3", "d2").expect("open d3");
    engine
        .commit(commit(
            Some("d2"),
            "d2b",
            3,
            "Chapter later",
            vec![holding("h-d2-late", "alice", 3, None)],
            vec![],
        ))
        .expect("d2 after d3");
    engine
        .commit(commit(
            Some("d3"),
            "d3a",
            3,
            "Scene",
            vec![fact("f-d3", "bob", 3)],
            vec![],
        ))
        .expect("d3");

    let view = |engine: &TestEngine| {
        let holders = ["alice", "bob", "carol"].map(|holder| holding_ids(engine, "d3", holder));
        let events = engine
            .in_timeline("d3")
            .timeline("p", At::default(), Page::default())
            .expect("events")
            .items
            .into_iter()
            .map(|event| event.id)
            .collect::<Vec<_>>();
        (holders, events)
    };
    let before = view(&engine);
    assert_eq!(before.0[0], ["h-main"], "d2 after d3 opened is invisible");

    engine
        .promote_draft("p", "d2")
        .expect("promote the middle draft");
    let d3 = engine.get_timeline("p", "d3").expect("d3");
    assert_eq!(d3.parent_timeline_id.as_deref(), Some("d1"));
    assert_eq!(d3.depth, 2);
    assert_eq!(view(&engine), before);
    let promoted_item = engine
        .changes_in("p", 0, 100, Some("d1"))
        .expect("changes")
        .items
        .into_iter()
        .find(|item| item.change_type == "timeline.promoted")
        .expect("promoted item");
    assert_eq!(
        promoted_item.payload["reparented"],
        serde_json::json!(["d3"])
    );

    // d1 now holds d2's later commit; d3 still cannot see it.
    assert!(holding_ids(&engine, "d1", "alice").contains(&"h-d2-late".to_owned()));
    assert_code(
        engine
            .promote_draft("p", "d3")
            .expect_err("d1 moved past d3's base"),
        UmweltErrorCode::VersionConflict,
    );
}

#[test]
fn drafts_that_others_depend_on_cannot_be_discarded_purged_or_rebased() {
    let mut engine = engine();
    open(&mut engine, "d1");
    open_from(&mut engine, "d2", "d1").expect("open d2");
    engine
        .commit(commit(
            None,
            "m1",
            1,
            "Main moves",
            vec![fact("f1", "alice", 1)],
            vec![],
        ))
        .expect("main moves");
    for result in [
        engine.discard_draft("p", "d1", false).map(|_| ()),
        engine.discard_draft("p", "d1", true).map(|_| ()),
        engine.rebase_draft("p", "d1").map(|_| ()),
    ] {
        let error = assert_code(
            result.expect_err("d2 depends on d1"),
            UmweltErrorCode::InvalidState,
        );
        assert_eq!(
            error.details.expect("details")["descendants"],
            serde_json::json!(["d2"])
        );
    }
    assert_eq!(
        engine.get_timeline("p", "d1").expect("d1").status,
        TimelineStatus::Active
    );

    engine
        .discard_draft("p", "d2", false)
        .expect("discard the child");
    engine
        .discard_draft("p", "d1", false)
        .expect("discard the parent");
    assert!(
        engine.in_timeline("d2").record("p", "f1").is_err(),
        "d2 still reads d1, not main's later commit"
    );
    assert_code(
        engine
            .purge_draft("p", "d1")
            .expect_err("d2 still reads d1"),
        UmweltErrorCode::InvalidState,
    );
    engine.purge_draft("p", "d2").expect("purge the child");
    engine.purge_draft("p", "d1").expect("purge the parent");
}

/// An attempt applied in an ancestor is replayed, not applied again.
#[test]
fn attempts_are_found_through_the_whole_chain() {
    let mut engine = engine();
    open(&mut engine, "d1");
    let first = engine
        .commit(commit(
            Some("d1"),
            "same",
            1,
            "Day",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("d1 commit");
    open_from(&mut engine, "d2", "d1").expect("open d2");
    open_from(&mut engine, "d3", "d2").expect("open d3");
    let counts = engine.counts("p").expect("counts");
    let replay = engine
        .commit(commit(
            Some("d3"),
            "same",
            1,
            "Day",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("replay in d3");
    assert_eq!(replay, first);
    assert_eq!(engine.counts("p").expect("counts"), counts);
}

#[test]
fn recorded_versions_are_unique_and_increasing_across_timelines() {
    let mut engine = engine();
    let mut versions = Vec::new();
    versions.push(
        engine
            .commit(commit(
                None,
                "m1",
                1,
                "a",
                vec![fact("f1", "alice", 1)],
                vec![],
            ))
            .expect("m1")
            .version,
    );
    open(&mut engine, "d");
    versions.push(
        engine
            .commit(commit(
                Some("d"),
                "d1",
                1,
                "b",
                vec![fact("f2", "bob", 1)],
                vec![],
            ))
            .expect("d1")
            .version,
    );
    versions.push(
        engine
            .commit(commit(
                None,
                "m2",
                1,
                "c",
                vec![fact("f3", "carol", 1)],
                vec![],
            ))
            .expect("m2")
            .version,
    );
    versions.push(
        engine
            .commit(commit(
                Some("d"),
                "d2",
                1,
                "d",
                vec![fact("f4", "alice", 1)],
                vec![],
            ))
            .expect("d2")
            .version,
    );
    assert_eq!(versions, [1, 2, 3, 4]);
    assert_eq!(engine.project("p").expect("project").version, 3);
    assert_eq!(
        engine.get_timeline("p", "d").expect("draft").head_version,
        4
    );
    let draft = engine
        .in_timeline("d")
        .record("p", "f4")
        .expect("draft record");
    assert!(
        matches!(draft, Record::Fact { common, .. } if common.recorded_at == 4 && common.timeline_id == "d")
    );
    assert_code(
        engine
            .in_timeline("d")
            .canon(
                "p",
                CanonFilter::default(),
                At {
                    axis: None,
                    tick: None,
                    as_of_version: Some(5),
                },
                Page::default(),
            )
            .expect_err("asOfVersion beyond the draft head"),
        UmweltErrorCode::ValidationFailed,
    );
}

#[test]
fn proposals_targeting_a_draft_apply_into_the_draft() {
    let mut engine = engine();
    open(&mut engine, "d");
    let proposal = engine
        .propose(ProposeInput {
            project_id: "p".to_owned(),
            timeline_id: Some("d".to_owned()),
            expected_version: None,
            event: commit(None, "x", 1, "Proposed", vec![], vec![]).event,
            records: vec![holding("h1", "alice", 1, None)],
            transitions: Vec::new(),
        })
        .expect("propose");
    engine
        .review(
            &proposal.id,
            vec![ReviewInput {
                operation_index: 0,
                decision: ReviewDecision::Accepted,
                rationale: "Fine".to_owned(),
            }],
        )
        .expect("review");
    let outcome = engine
        .apply(&proposal.id, "apply-1".to_owned())
        .expect("apply");
    assert_eq!(outcome.record_ids, ["h1"]);
    assert!(holds(&engine, "d", "alice"));
    assert!(!holds(&engine, "main", "alice"));
}

#[test]
fn promoted_drafts_redirect_to_main_and_purged_drafts_are_gone() {
    let mut engine = engine();
    open(&mut engine, "promoted");
    engine
        .commit(commit(
            Some("promoted"),
            "a1",
            1,
            "Day",
            vec![holding("h1", "alice", 1, None)],
            vec![],
        ))
        .expect("draft commit");
    let timeline = engine.promote_draft("p", "promoted").expect("promote");
    let error = assert_code(
        engine
            .in_timeline("promoted")
            .holdings(
                "p",
                "alice",
                HoldingFilter::default(),
                At::default(),
                Page::default(),
            )
            .expect_err("promoted drafts are read through main"),
        UmweltErrorCode::InvalidState,
    );
    let details = error.details.expect("details");
    assert_eq!(details["promotedInto"], "main");
    assert_eq!(details["headVersion"], timeline.head_version);
    assert_code(
        engine
            .commit(commit(
                Some("promoted"),
                "a2",
                2,
                "Late",
                vec![fact("f1", "alice", 2)],
                vec![],
            ))
            .expect_err("promoted drafts accept no writes"),
        UmweltErrorCode::InvalidState,
    );
    assert_code(
        engine
            .discard_draft("p", "promoted", true)
            .expect_err("cannot discard promoted"),
        UmweltErrorCode::InvalidState,
    );
    assert_code(
        engine
            .purge_draft("p", "promoted")
            .expect_err("cannot purge promoted"),
        UmweltErrorCode::InvalidState,
    );
    assert_code(
        engine
            .purge_draft("p", "main")
            .expect_err("cannot purge main"),
        UmweltErrorCode::InvalidState,
    );

    open(&mut engine, "gone");
    engine
        .discard_draft("p", "gone", true)
        .expect("discard and purge");
    assert_code(
        engine
            .in_timeline("gone")
            .trace("p", "h1")
            .expect_err("purged drafts cannot be read"),
        UmweltErrorCode::NotFound,
    );
    assert_code(
        engine
            .in_timeline("missing")
            .record("p", "h1")
            .expect_err("unknown timeline"),
        UmweltErrorCode::NotFound,
    );
}

/// The client's working day: everything of the day goes to a draft; quitting
/// without saving discards and purges it; the day is played again with the
/// same ids and attempts and promoted when the player sleeps.
fn play_day(engine: &mut TestEngine, timeline: &str) {
    engine
        .commit_batch(vec![
            commit(
                Some(timeline),
                "day2-1",
                2,
                "Market",
                vec![holding("day2-alice", "alice", 2, Some("lantern"))],
                vec![],
            ),
            commit(
                Some(timeline),
                "day2-2",
                2,
                "Rumor",
                vec![fact("day2-fact", "carol", 2)],
                vec![forget("day2-forget", "day1-carol", 2)],
            ),
        ])
        .expect("day commits");
    engine
        .transmit(TransmitInput {
            on: Default::default(),
            project_id: "p".to_owned(),
            attempt_id: "day2-gossip".to_owned(),
            timeline_id: Some(timeline.to_owned()),
            position: Position { tick: 2 },
            from: "alice".to_owned(),
            to: "bob".to_owned(),
            source_record_id: "day2-alice".to_owned(),
            record_id: Some("day2-bob".to_owned()),
            attitude: Attitude::Believes,
            confidence: 0.6,
            intent: Intent::Honest,
            distorted_claim: None,
            text: None,
        })
        .expect("gossip");
}

fn first_day(engine: &mut TestEngine) {
    engine
        .commit(commit(
            None,
            "day1",
            1,
            "First day",
            vec![holding("day1-carol", "carol", 1, None)],
            vec![],
        ))
        .expect("first day");
}

/// Main as a client sees it, without recording versions (they differ when a
/// draft was discarded first) and without the ids the engine generates for
/// transmit events (a transmit carries no event id, so a replayed day gets new
/// ones).
fn main_state(engine: &TestEngine) -> Value {
    let mut state = serde_json::Map::new();
    for holder in ["alice", "bob", "carol"] {
        let holdings = engine
            .holdings(
                "p",
                holder,
                HoldingFilter::default(),
                At::default(),
                Page::default(),
            )
            .expect("holdings");
        state.insert(
            holder.to_owned(),
            serde_json::to_value(holdings).expect("json"),
        );
    }
    let canon = engine
        .canon("p", CanonFilter::default(), At::default(), Page::default())
        .expect("canon");
    state.insert(
        "canon".to_owned(),
        serde_json::to_value(canon).expect("json"),
    );
    let events = engine
        .timeline("p", At::default(), Page::default())
        .expect("events");
    state.insert(
        "events".to_owned(),
        serde_json::to_value(events).expect("json"),
    );
    let trace = engine.trace("p", "day1-carol").expect("trace");
    state.insert(
        "trace".to_owned(),
        serde_json::to_value(trace).expect("json"),
    );
    let mut value = Value::Object(state);
    strip_versions(&mut value);
    value
}

fn strip_versions(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.remove("recordedAt");
            fields.remove("nextCursor");
            fields.values_mut().for_each(strip_versions);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_versions),
        Value::String(text) if text.starts_with("id-") => *text = "generated".to_owned(),
        _ => {}
    }
}

#[test]
fn pending_day_discarded_then_replayed_equals_the_day_applied_to_main() {
    let mut drafted = engine();
    first_day(&mut drafted);
    open(&mut drafted, "day-2-try-1");
    play_day(&mut drafted, "day-2-try-1");
    drafted
        .discard_draft("p", "day-2-try-1", true)
        .expect("quit without saving");
    open(&mut drafted, "day-2-try-2");
    play_day(&mut drafted, "day-2-try-2");
    drafted.promote_draft("p", "day-2-try-2").expect("sleep");

    let mut direct = engine();
    first_day(&mut direct);
    play_day(&mut direct, MAIN_TIMELINE_ID);

    assert_eq!(main_state(&drafted), main_state(&direct));
    // The purged attempt left nothing behind.
    assert_eq!(
        drafted.counts("p").expect("counts"),
        direct.counts("p").expect("counts")
    );
    assert!(
        drafted
            .who_holds(
                "p",
                ClaimRef::Claim {
                    claim: claim("alice")
                },
                At::default(),
                WhoHoldsFilter::default(),
            )
            .expect("who holds")
            .iter()
            .any(|item| item.holder == "bob")
    );
}
