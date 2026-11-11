// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::json;
use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore, claim_key};
use umwelt_core::domain::*;

type TestEngine = Engine<SqliteStore>;

fn provenance() -> Provenance {
    Provenance {
        origin: Origin::System {
            name: "queries-test".to_owned(),
        },
        reason: None,
        metadata: None,
    }
}

fn setup() -> TestEngine {
    let mut engine = Engine::new(
        SqliteStore::in_memory().expect("create store"),
        Arc::new(FixedClock("2026-09-29T12:00:00Z".to_owned())),
        Arc::new(SeqIdGen::new()),
    );
    engine
        .put_project(Project {
            id: "p".to_owned(),
            name: "Queries".to_owned(),
            description: None,
            timescale: "tick".to_owned(),
            vocabulary_mode: VocabularyMode::Strict,
            created_at: "2026-09-29T12:00:00Z".to_owned(),
            version: 0,
        })
        .expect("create project");
    for (id, kind) in [("agent", EntityKind::Agent), ("other", EntityKind::Other)] {
        engine
            .put_entity_type(EntityType {
                id: id.to_owned(),
                project_id: "p".to_owned(),
                name: id.to_owned(),
                kind,
                fields: Vec::new(),
            })
            .expect("put entity type");
    }
    for (id, type_id) in [
        ("alice", "agent"),
        ("bob", "agent"),
        ("carol", "agent"),
        ("team", "other"),
        ("item", "other"),
        ("place", "other"),
    ] {
        engine
            .put_entity(Entity {
                id: id.to_owned(),
                project_id: "p".to_owned(),
                type_id: type_id.to_owned(),
                name: id.to_owned(),
                aliases: Vec::new(),
                fields: serde_json::Map::new(),
                external_ref: None,
                created_at: Position { tick: 0 },
                archived: false,
            })
            .expect("put entity");
    }
    for (id, cardinality) in [
        ("related", Cardinality::Many),
        ("location", Cardinality::One),
    ] {
        engine
            .put_predicate(
                "p",
                PredicateDef {
                    id: id.to_owned(),
                    label: id.to_owned(),
                    object: ObjectKind::Entity,
                    cardinality,
                    subject_types: None,
                },
            )
            .expect("put predicate");
    }
    engine
}

fn claim(subject: &str, predicate: &str, object: &str, polarity: bool) -> Claim {
    Claim {
        subject: subject.to_owned(),
        predicate: predicate.to_owned(),
        object: ClaimObject::Entity {
            id: object.to_owned(),
        },
        qualifiers: BTreeMap::new(),
        polarity,
        about: None,
    }
}

fn common(id: &str, tick: i64) -> RecordInputCommon {
    RecordInputCommon {
        id: Some(id.to_owned()),
        position: Some(Position { tick }),
        provenance: Some(provenance()),
    }
}

fn holding(
    id: &str,
    holder: &str,
    tick: i64,
    value: Claim,
    accessibility: Accessibility,
    acquisition: Acquisition,
    text: Option<&str>,
) -> RecordInput {
    holding_with_attitude(
        id,
        holder,
        tick,
        value,
        Attitude::Believes,
        accessibility,
        acquisition,
        text,
    )
}

#[allow(clippy::too_many_arguments)]
fn holding_with_attitude(
    id: &str,
    holder: &str,
    tick: i64,
    value: Claim,
    attitude: Attitude,
    accessibility: Accessibility,
    acquisition: Acquisition,
    text: Option<&str>,
) -> RecordInput {
    RecordInput::Holding {
        common: common(id, tick),
        holder: holder.to_owned(),
        attitude,
        claim: value,
        acquisition,
        confidence: 0.8,
        accessibility,
        text: text.map(str::to_owned),
        secrecy: None,
    }
}

#[test]
fn search_escapes_fts_syntax_and_preserves_diacritic_matching() {
    let mut engine = setup();
    engine
        .commit(CommitInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            attempt_id: "search-safe".to_owned(),
            expected_version: None,
            event: EventInput {
                on: Default::default(),
                id: Some("search-event".to_owned()),
                event_type: "test".to_owned(),
                title: "Mira's lantern lantern-gift Theo AND \"unbalanced é".to_owned(),
                description: None,
                position: Position { tick: 1 },
                participants: Vec::new(),
                location: None,
                evidence: Vec::new(),
                provenance: provenance(),
            },
            records: vec![RecordInput::Fact {
                common: common("search-fact", 1),
                claim: claim("alice", "related", "item", true),
            }],
            transitions: Vec::new(),
            preconditions: Vec::new(),
        })
        .expect("commit search fixture");
    for text in ["Mira's lantern", "lantern-gift", "Theo AND", "\"unbalanced"] {
        let result = engine
            .search(
                "p",
                text,
                SearchFilter::default(),
                At::default(),
                Page::default(),
            )
            .expect("safe search input");
        assert_eq!(result.items.len(), 1, "{text}");
    }
    let empty = engine
        .search(
            "p",
            "   ",
            SearchFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("empty search");
    assert!(empty.items.is_empty());
    let accented = engine
        .search(
            "p",
            "é",
            SearchFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("accented search");
    let plain = engine
        .search(
            "p",
            "e",
            SearchFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("plain search");
    assert_eq!(
        accented
            .items
            .iter()
            .map(|item| &item.doc_id)
            .collect::<Vec<_>>(),
        plain
            .items
            .iter()
            .map(|item| &item.doc_id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn multiple_attitudes_are_filtered_before_stable_pagination() {
    let mut engine = setup();
    let records = (0..8)
        .map(|index| {
            let attitude = match index % 3 {
                0 => Attitude::Believes,
                1 => Attitude::Suspects,
                _ => Attitude::Knows,
            };
            let mut value = claim("alice", "related", "item", true);
            value.qualifiers.insert("index".to_owned(), json!(index));
            holding_with_attitude(
                &format!("paged-{index}"),
                "alice",
                2,
                value,
                attitude,
                Accessibility::Available,
                Acquisition::Authored,
                None,
            )
        })
        .collect();
    commit(&mut engine, "paged", "paged-event", 2, records);
    let filter = HoldingFilter {
        attitude: vec![Attitude::Believes, Attitude::Suspects],
        ..HoldingFilter::default()
    };
    let complete = engine
        .holdings("p", "alice", filter.clone(), At::default(), Page::default())
        .expect("complete filtered holdings")
        .items
        .into_iter()
        .map(|item| match item.record {
            Record::Holding { common, .. } => common.id,
            _ => unreachable!("holdings query returned a non-holding"),
        })
        .collect::<Vec<_>>();
    let mut cursor = None;
    let mut paged = Vec::new();
    for _ in 0..3 {
        let result = engine
            .holdings(
                "p",
                "alice",
                filter.clone(),
                At::default(),
                Page { limit: 2, cursor },
            )
            .expect("filtered holdings page");
        paged.extend(result.items.into_iter().map(|item| match item.record {
            Record::Holding { common, .. } => common.id,
            _ => unreachable!("holdings query returned a non-holding"),
        }));
        cursor = result.next_cursor;
    }
    assert_eq!(paged, complete);
}

fn commit(
    engine: &mut TestEngine,
    attempt: &str,
    event_id: &str,
    tick: i64,
    records: Vec<RecordInput>,
) {
    engine
        .commit(CommitInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            attempt_id: attempt.to_owned(),
            expected_version: None,
            event: EventInput {
                on: Default::default(),
                id: Some(event_id.to_owned()),
                event_type: "test".to_owned(),
                title: format!("Lantern event {event_id}"),
                description: None,
                position: Position { tick },
                participants: Vec::new(),
                location: None,
                evidence: Vec::new(),
                provenance: provenance(),
            },
            records,
            transitions: Vec::new(),
            preconditions: Vec::new(),
        })
        .expect("commit records");
}

#[test]
fn holds_holdings_who_holds_and_canon_respect_time_and_withholding() {
    let mut engine = setup();
    let value = claim("alice", "related", "item", true);
    commit(
        &mut engine,
        "a1",
        "e1",
        1,
        vec![
            RecordInput::Fact {
                common: common("fact", 1),
                claim: value.clone(),
            },
            holding(
                "alice-holding",
                "alice",
                1,
                value.clone(),
                Accessibility::Available,
                Acquisition::Authored,
                Some("visible lantern"),
            ),
            holding(
                "bob-secret",
                "bob",
                1,
                value.clone(),
                Accessibility::Suppressed,
                Acquisition::Authored,
                Some("secret lantern"),
            ),
        ],
    );
    commit(
        &mut engine,
        "a2",
        "e2",
        2,
        vec![holding(
            "alice-later",
            "alice",
            2,
            claim("alice", "related", "place", true),
            Accessibility::Available,
            Acquisition::Authored,
            None,
        )],
    );
    assert!(
        engine
            .holds(
                "p",
                "alice",
                ClaimRef::Claim {
                    claim: value.clone(),
                },
                At::default(),
            )
            .expect("holds")
            .holds
    );
    assert!(
        !engine
            .holds(
                "p",
                "bob",
                ClaimRef::ClaimKey {
                    claim_key: claim_key(&value),
                },
                At::default(),
            )
            .expect("withheld holds")
            .holds
    );
    let hidden = engine
        .holdings(
            "p",
            "bob",
            HoldingFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("hidden holdings");
    assert!(hidden.items.is_empty());
    assert_eq!(hidden.withheld_count, 1);
    let visible = engine
        .holdings(
            "p",
            "bob",
            HoldingFilter {
                include_withheld: true,
                ..HoldingFilter::default()
            },
            At::default(),
            Page::default(),
        )
        .expect("included withheld holdings");
    assert_eq!(visible.items.len(), 1);
    assert_eq!(
        engine
            .who_holds(
                "p",
                ClaimRef::Claim { claim: value },
                At::default(),
                WhoHoldsFilter::default(),
            )
            .expect("who holds")
            .len(),
        1
    );
    assert_eq!(
        engine
            .canon("p", CanonFilter::default(), At::default(), Page::default())
            .expect("canon")
            .items
            .len(),
        1
    );
    assert_eq!(
        engine
            .holdings(
                "p",
                "alice",
                HoldingFilter::default(),
                At {
                    axis: None,
                    tick: None,
                    as_of_version: Some(1),
                },
                Page::default(),
            )
            .expect("historical holdings")
            .items
            .len(),
        1
    );
    let unscoped = engine
        .search(
            "p",
            "visible",
            SearchFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("unscoped search");
    assert!(unscoped.items.is_empty());
    let scoped = engine
        .search(
            "p",
            "visible",
            SearchFilter {
                holder: Some("alice".to_owned()),
                doc_type: Some("record".to_owned()),
            },
            At::default(),
            Page::default(),
        )
        .expect("holder-scoped search");
    assert_eq!(scoped.items.len(), 1);
}

#[test]
fn perspective_compare_and_both_contradiction_kinds_are_reported() {
    let mut engine = setup();
    let canonical = claim("alice", "related", "item", true);
    commit(
        &mut engine,
        "state",
        "state-event",
        3,
        vec![
            RecordInput::Fact {
                common: common("canon", 3),
                claim: canonical.clone(),
            },
            RecordInput::Fact {
                common: common("membership", 3),
                claim: claim("alice", MEMBER_OF_PREDICATE_ID, "team", true),
            },
            holding(
                "alice-opposite",
                "alice",
                3,
                claim("alice", "related", "item", false),
                Accessibility::Available,
                Acquisition::Authored,
                None,
            ),
            holding(
                "alice-location-a",
                "alice",
                3,
                claim("alice", "location", "item", true),
                Accessibility::Available,
                Acquisition::Authored,
                None,
            ),
            holding(
                "alice-location-b",
                "alice",
                3,
                claim("alice", "location", "place", true),
                Accessibility::Available,
                Acquisition::Authored,
                None,
            ),
            holding(
                "bob-shared",
                "bob",
                3,
                claim("alice", "location", "item", true),
                Accessibility::Available,
                Acquisition::Authored,
                None,
            ),
            RecordInput::Awareness {
                common: common("aware", 3),
                holder: "alice".to_owned(),
                target: "bob".to_owned(),
            },
            RecordInput::Disposition {
                common: common("disposition", 3),
                holder: "alice".to_owned(),
                target: "bob".to_owned(),
                dimension: "trust".to_owned(),
                value: json!(0.4),
            },
        ],
    );
    commit(
        &mut engine,
        "state-update",
        "state-update-event",
        4,
        vec![RecordInput::Disposition {
            common: common("disposition-later", 4),
            holder: "alice".to_owned(),
            target: "bob".to_owned(),
            dimension: "trust".to_owned(),
            value: json!(0.9),
        }],
    );
    let perspective = engine
        .perspective("p", "alice", At::default(), Page::default())
        .expect("perspective");
    assert_eq!(perspective.holder.id, "alice");
    assert_eq!(perspective.awareness.len(), 1);
    assert_eq!(perspective.dispositions.len(), 1);
    assert!(matches!(
        &perspective.dispositions[0],
        Record::Disposition { value, .. } if value == &json!(0.9)
    ));
    assert_eq!(perspective.memberships.len(), 1);
    assert!(
        perspective
            .holdings
            .iter()
            .all(|item| matches!(item.record, Record::Holding { .. }))
    );
    let compared = engine
        .compare("p", "alice", "bob", At::default())
        .expect("compare");
    assert_eq!(compared.counts.shared, 1);
    assert!(compared.counts.only_a >= 1);
    let contradictions = engine
        .contradictions("p", "alice", At::default())
        .expect("contradictions");
    assert!(
        contradictions
            .iter()
            .any(|item| item.kind == ContradictionKind::VsCanon)
    );
    assert!(contradictions.iter().any(|item| {
        item.kind == ContradictionKind::VsSelf
            && item.reason == ContradictionReason::DifferentValueInFunctionalSlot
    }));
}

#[test]
fn trace_follows_three_links_and_stops_on_cycles() {
    let mut engine = setup();
    let value = claim("alice", "related", "item", true);
    commit(
        &mut engine,
        "trace",
        "trace-event",
        4,
        vec![
            holding(
                "root",
                "alice",
                4,
                value.clone(),
                Accessibility::Available,
                Acquisition::Authored,
                None,
            ),
            holding(
                "middle",
                "bob",
                4,
                value.clone(),
                Accessibility::Available,
                Acquisition::ToldBy {
                    source_holder_id: "alice".to_owned(),
                    source_record_id: Some("root".to_owned()),
                    intent: Intent::Honest,
                },
                None,
            ),
            holding(
                "leaf",
                "carol",
                4,
                value.clone(),
                Accessibility::Available,
                Acquisition::ToldBy {
                    source_holder_id: "bob".to_owned(),
                    source_record_id: Some("middle".to_owned()),
                    intent: Intent::Honest,
                },
                None,
            ),
            holding(
                "cycle-a",
                "alice",
                4,
                value.clone(),
                Accessibility::Available,
                Acquisition::ToldBy {
                    source_holder_id: "bob".to_owned(),
                    source_record_id: Some("cycle-b".to_owned()),
                    intent: Intent::Unknown,
                },
                None,
            ),
            holding(
                "cycle-b",
                "bob",
                4,
                value,
                Accessibility::Available,
                Acquisition::ToldBy {
                    source_holder_id: "alice".to_owned(),
                    source_record_id: Some("cycle-a".to_owned()),
                    intent: Intent::Unknown,
                },
                None,
            ),
        ],
    );
    let trace = engine.trace("p", "leaf").expect("trace chain");
    assert_eq!(trace.chain.len(), 3);
    assert!(!trace.truncated);
    let cycle = engine.trace("p", "cycle-a").expect("trace cycle");
    assert_eq!(cycle.chain.len(), 2);
    assert!(cycle.truncated);
}

#[test]
fn timeline_pagination_record_entities_predicates_changes_and_search_work() {
    let mut engine = setup();
    for index in 1..=7 {
        commit(
            &mut engine,
            &format!("attempt-{index}"),
            &format!("event-{index}"),
            index,
            vec![RecordInput::Fact {
                common: common(&format!("fact-{index}"), index),
                claim: claim("alice", "related", "item", true),
            }],
        );
    }
    let mut cursor = None;
    let mut paged = Vec::new();
    for _ in 0..3 {
        let result = engine
            .timeline("p", At::default(), Page { limit: 3, cursor })
            .expect("timeline page");
        paged.extend(result.items.iter().map(|event| event.id.clone()));
        cursor = result.next_cursor;
    }
    let complete = engine
        .timeline(
            "p",
            At::default(),
            Page {
                limit: 100,
                cursor: None,
            },
        )
        .expect("complete timeline")
        .items
        .into_iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    assert_eq!(paged, complete);
    assert!(matches!(
        engine.record("p", "fact-1").expect("record"),
        Record::Fact { common, .. } if common.id == "fact-1"
    ));
    assert_eq!(engine.entity("p", "alice").expect("entity").id, "alice");
    assert_eq!(
        engine
            .entities("p", Page::default())
            .expect("entities")
            .items
            .len(),
        6
    );
    // Two defined by the test plus six reserved `umwelt:` predicates.
    assert_eq!(engine.predicates("p").expect("predicates").len(), 8);
    assert_eq!(engine.changes("p", 0, 100).expect("changes").items.len(), 7);
    let search = engine
        .search(
            "p",
            "Lantern",
            SearchFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("search events");
    assert_eq!(search.items.len(), 7);
    assert!(search.items.iter().all(|item| item.event.is_some()));
    let facts = engine
        .search(
            "p",
            "alice",
            SearchFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("search facts");
    assert_eq!(facts.items.len(), 7);
    assert!(facts.items.iter().all(|item| item.record.is_some()));
}
