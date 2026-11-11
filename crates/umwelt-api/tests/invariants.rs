// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::json;
use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore};
use umwelt_core::domain::*;
use umwelt_core::{RecordQuery, StoreCounts, UmweltErrorCode, claim_key as calculate_claim_key};

type TestEngine = Engine<SqliteStore>;

fn provenance() -> Provenance {
    Provenance {
        origin: Origin::Human { actor_id: None },
        reason: None,
        metadata: None,
    }
}

fn engine() -> TestEngine {
    let store = SqliteStore::in_memory().expect("create in-memory store");
    let mut engine = Engine::new(
        store,
        Arc::new(FixedClock("2026-09-28T12:00:00Z".to_owned())),
        Arc::new(SeqIdGen::new()),
    );
    engine
        .put_project(Project {
            id: "p".to_owned(),
            name: "Test".to_owned(),
            description: None,
            timescale: "tick".to_owned(),
            vocabulary_mode: VocabularyMode::Open,
            created_at: "2026-09-28T12:00:00Z".to_owned(),
            version: 0,
        })
        .expect("create project");
    for (id, kind) in [
        ("agent", EntityKind::Agent),
        ("group", EntityKind::Group),
        ("other", EntityKind::Other),
    ] {
        engine
            .put_entity_type(EntityType {
                id: id.to_owned(),
                project_id: "p".to_owned(),
                name: id.to_owned(),
                kind,
                fields: Vec::new(),
            })
            .expect("create entity type");
    }
    for (id, type_id) in [
        ("alice", "agent"),
        ("bob", "agent"),
        ("team", "group"),
        ("place", "other"),
        ("item", "other"),
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
            .expect("create entity");
    }
    engine
        .put_predicate(
            "p",
            PredicateDef {
                id: "related".to_owned(),
                label: "Related".to_owned(),
                object: ObjectKind::Entity,
                cardinality: Cardinality::Many,
                subject_types: None,
            },
        )
        .expect("create predicate");
    engine
}

fn claim(subject: &str, object: &str, polarity: bool) -> Claim {
    Claim {
        subject: subject.to_owned(),
        predicate: "related".to_owned(),
        object: ClaimObject::Entity {
            id: object.to_owned(),
        },
        qualifiers: BTreeMap::new(),
        polarity,
        about: None,
    }
}

fn event(id: &str, tick: i64, participants: Vec<&str>) -> EventInput {
    EventInput {
        on: Default::default(),
        id: Some(id.to_owned()),
        event_type: "test".to_owned(),
        title: "Test".to_owned(),
        description: None,
        position: Position { tick },
        participants: participants.into_iter().map(str::to_owned).collect(),
        location: None,
        evidence: Vec::new(),
        provenance: provenance(),
    }
}

fn common(id: &str, tick: i64) -> RecordInputCommon {
    RecordInputCommon {
        id: Some(id.to_owned()),
        position: Some(Position { tick }),
        provenance: Some(provenance()),
    }
}

fn holding(id: &str, holder: &str, tick: i64, value: Claim) -> RecordInput {
    RecordInput::Holding {
        common: common(id, tick),
        holder: holder.to_owned(),
        attitude: Attitude::Believes,
        claim: value,
        acquisition: Acquisition::Authored,
        confidence: 1.0,
        accessibility: Accessibility::Available,
        text: None,
        secrecy: None,
    }
}

fn commit(records: Vec<RecordInput>, attempt: &str, event_id: &str, tick: i64) -> CommitInput {
    CommitInput {
        timeline_id: None,
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        expected_version: None,
        event: event(event_id, tick, Vec::new()),
        records,
        transitions: Vec::new(),
        preconditions: Vec::new(),
    }
}

#[test]
fn fact_never_appears_in_a_holder_projection() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![RecordInput::Fact {
                common: common("fact", 1),
                claim: claim("alice", "item", true),
            }],
            "a1",
            "e1",
            1,
        ))
        .expect("commit fact");
    assert!(
        engine
            .holder_projection("p", "alice", 1, 1)
            .expect("read projection")
            .is_empty()
    );
}

#[test]
fn event_participants_do_not_grant_records() {
    let mut engine = engine();
    let mut input = commit(
        vec![RecordInput::Fact {
            common: common("fact", 1),
            claim: claim("alice", "item", true),
        }],
        "a1",
        "e1",
        1,
    );
    input.event.participants = vec!["alice".to_owned()];
    engine.commit(input).expect("commit event");
    assert!(
        engine
            .holder_projection("p", "alice", 1, 1)
            .expect("read projection")
            .is_empty()
    );
}

#[test]
fn awareness_reveals_only_identity() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![RecordInput::Awareness {
                common: common("aware", 1),
                holder: "alice".to_owned(),
                target: "bob".to_owned(),
            }],
            "a1",
            "e1",
            1,
        ))
        .expect("commit awareness");
    let projection = engine
        .holder_projection("p", "alice", 1, 1)
        .expect("read projection");
    assert!(matches!(&projection[..], [Record::Awareness { target, .. }] if target == "bob"));
}

#[test]
fn belief_that_contradicts_canon_remains_active() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![
                RecordInput::Fact {
                    common: common("fact", 1),
                    claim: claim("alice", "item", true),
                },
                holding("belief", "alice", 1, claim("alice", "item", false)),
            ],
            "a1",
            "e1",
            1,
        ))
        .expect("commit records");
    let projection = engine
        .holder_projection("p", "alice", 1, 1)
        .expect("read projection");
    assert!(matches!(&projection[..], [Record::Holding { claim, .. }] if !claim.polarity));
}

#[test]
fn holders_are_isolated() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![
                holding("alice-belief", "alice", 1, claim("alice", "item", true)),
                holding("bob-belief", "bob", 1, claim("bob", "item", true)),
            ],
            "a1",
            "e1",
            1,
        ))
        .expect("commit holdings");
    let alice = engine
        .holder_projection("p", "alice", 1, 1)
        .expect("alice projection");
    assert_eq!(alice.len(), 1);
    assert!(matches!(&alice[0], Record::Holding { holder, .. } if holder == "alice"));
}

#[test]
fn group_holding_is_not_the_union_of_member_holdings() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![
                holding("group-belief", "team", 1, claim("team", "item", true)),
                holding("member-belief", "alice", 1, claim("alice", "item", true)),
            ],
            "a1",
            "e1",
            1,
        ))
        .expect("commit holdings");
    assert_eq!(
        engine
            .holder_projection("p", "team", 1, 1)
            .expect("group projection")
            .len(),
        1
    );
    assert_eq!(
        engine
            .holder_projection("p", "alice", 1, 1)
            .expect("member projection")
            .len(),
        1
    );
}

#[test]
fn disposition_is_directional() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![RecordInput::Disposition {
                common: common("d1", 1),
                holder: "alice".to_owned(),
                target: "bob".to_owned(),
                dimension: "trust".to_owned(),
                value: json!(0.8),
            }],
            "a1",
            "e1",
            1,
        ))
        .expect("commit disposition");
    assert_eq!(
        engine
            .holder_projection("p", "alice", 1, 1)
            .expect("alice projection")
            .len(),
        1
    );
    assert!(
        engine
            .holder_projection("p", "bob", 1, 1)
            .expect("bob projection")
            .is_empty()
    );
}

#[test]
fn future_record_is_invisible_at_an_earlier_tick() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("future", "alice", 10, claim("alice", "item", true))],
            "a1",
            "e1",
            10,
        ))
        .expect("commit future holding");
    assert!(
        engine
            .holder_projection("p", "alice", 9, 1)
            .expect("read earlier tick")
            .is_empty()
    );
}

#[test]
fn transition_preserves_history_at_an_earlier_tick() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("belief", "alice", 1, claim("alice", "item", true))],
            "a1",
            "e1",
            1,
        ))
        .expect("commit holding");
    let mut transition = commit(Vec::new(), "a2", "e2", 10);
    transition.transitions.push(TransitionInput {
        id: Some("t1".to_owned()),
        target_record_id: "belief".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick: 10 }),
        provenance: Some(provenance()),
    });
    engine.commit(transition).expect("commit transition");
    assert_eq!(
        engine
            .holder_projection("p", "alice", 9, 2)
            .expect("read history")
            .len(),
        1
    );
    assert!(
        engine
            .holder_projection("p", "alice", 10, 2)
            .expect("read current")
            .is_empty()
    );
}

#[test]
fn replay_returns_identical_outcome_without_new_rows() {
    let mut engine = engine();
    let input = commit(
        vec![holding("belief", "alice", 1, claim("alice", "item", true))],
        "attempt",
        "event",
        1,
    );
    let first = engine.commit(input.clone()).expect("first commit");
    let before = engine.counts("p").expect("count before replay");
    let replay = engine.commit(input).expect("replay commit");
    assert_eq!(first, replay);
    assert_eq!(before, engine.counts("p").expect("count after replay"));
}

#[test]
fn validation_failure_rolls_back_the_whole_commit() {
    let mut engine = engine();
    let before = engine.counts("p").expect("initial counts");
    let input = commit(
        vec![
            holding("valid", "alice", 1, claim("alice", "item", true)),
            holding("invalid", "missing", 1, claim("alice", "item", true)),
        ],
        "a1",
        "e1",
        1,
    );
    assert!(engine.commit(input).is_err());
    assert_eq!(before, engine.counts("p").expect("counts after failure"));
}

#[test]
fn rejected_proposal_never_enters_the_timeline() {
    let mut engine = engine();
    let proposal = engine
        .propose(ProposeInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            expected_version: None,
            event: event("e1", 1, Vec::new()),
            records: vec![holding("belief", "alice", 1, claim("alice", "item", true))],
            transitions: Vec::new(),
        })
        .expect("create proposal");
    engine
        .reject(&proposal.id, "Not suitable")
        .expect("reject proposal");
    assert!(
        engine
            .active_records("p", 1, 0)
            .expect("read timeline")
            .is_empty()
    );
    assert_eq!(
        engine.counts("p").expect("counts"),
        StoreCounts {
            events: 0,
            records: 0,
            transitions: 0,
            attempts: 0,
            proposals: 1
        }
    );
}

#[test]
fn retroactive_commit_does_not_change_an_earlier_version_read() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![RecordInput::Fact {
                common: common("fact", 10),
                claim: claim("bob", "item", true),
            }],
            "a1",
            "e1",
            10,
        ))
        .expect("first commit");
    engine
        .commit(commit(
            vec![holding("retro", "alice", 1, claim("alice", "item", true))],
            "a2",
            "e2",
            1,
        ))
        .expect("retroactive commit");
    assert!(
        engine
            .holder_projection("p", "alice", 10, 1)
            .expect("read old version")
            .is_empty()
    );
    assert_eq!(
        engine
            .holder_projection("p", "alice", 10, 2)
            .expect("read new version")
            .len(),
        1
    );
}

#[test]
fn transmit_creates_a_told_by_acquisition_pointing_to_the_source() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("source", "alice", 1, claim("alice", "item", true))],
            "a1",
            "e1",
            1,
        ))
        .expect("commit source");
    let outcome = engine
        .transmit(TransmitInput {
            on: Default::default(),
            timeline_id: None,
            project_id: "p".to_owned(),
            attempt_id: "a2".to_owned(),
            position: Position { tick: 2 },
            from: "alice".to_owned(),
            to: "bob".to_owned(),
            source_record_id: "source".to_owned(),
            record_id: None,
            attitude: Attitude::Believes,
            confidence: 0.8,
            intent: Intent::Honest,
            distorted_claim: None,
            text: None,
        })
        .expect("transmit");
    let transmitted = engine
        .record("p", &outcome.record_ids[0])
        .expect("read transmitted holding");
    assert!(
        matches!(transmitted, Record::Holding { holder, acquisition: Acquisition::ToldBy { source_holder_id, source_record_id: Some(source), .. }, .. } if holder == "bob" && source_holder_id == "alice" && source == "source")
    );
}

#[test]
fn transmit_rejects_an_inactive_source_or_a_different_holder() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("source", "alice", 1, claim("alice", "item", true))],
            "a1",
            "e1",
            1,
        ))
        .expect("commit source");
    let wrong_holder = TransmitInput {
        on: Default::default(),
        timeline_id: None,
        project_id: "p".to_owned(),
        attempt_id: "a2".to_owned(),
        position: Position { tick: 2 },
        from: "bob".to_owned(),
        to: "alice".to_owned(),
        source_record_id: "source".to_owned(),
        record_id: None,
        attitude: Attitude::Believes,
        confidence: 1.0,
        intent: Intent::Unknown,
        distorted_claim: None,
        text: None,
    };
    assert!(engine.transmit(wrong_holder).is_err());
    let mut transition = commit(Vec::new(), "a3", "e3", 2);
    transition.transitions.push(TransitionInput {
        id: Some("t1".to_owned()),
        target_record_id: "source".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick: 2 }),
        provenance: Some(provenance()),
    });
    engine.commit(transition).expect("deactivate source");
    let inactive = TransmitInput {
        on: Default::default(),
        timeline_id: None,
        project_id: "p".to_owned(),
        attempt_id: "a4".to_owned(),
        position: Position { tick: 3 },
        from: "alice".to_owned(),
        to: "bob".to_owned(),
        source_record_id: "source".to_owned(),
        record_id: None,
        attitude: Attitude::Believes,
        confidence: 1.0,
        intent: Intent::Unknown,
        distorted_claim: None,
        text: None,
    };
    assert!(engine.transmit(inactive).is_err());
}

#[test]
fn transitions_at_the_same_tick_use_version_then_ordinal_as_tiebreakers() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("source", "alice", 1, claim("alice", "item", true))],
            "a1",
            "e1",
            1,
        ))
        .expect("commit source");
    let mut input = commit(Vec::new(), "a2", "e2", 2);
    input.transitions = vec![
        TransitionInput {
            id: Some("t1".to_owned()),
            target_record_id: "source".to_owned(),
            status: TransitionStatus::Forgotten,
            superseded_by: None,
            position: Some(Position { tick: 2 }),
            provenance: Some(provenance()),
        },
        TransitionInput {
            id: Some("t2".to_owned()),
            target_record_id: "source".to_owned(),
            status: TransitionStatus::Revoked,
            superseded_by: None,
            position: Some(Position { tick: 2 }),
            provenance: Some(provenance()),
        },
    ];
    engine.commit(input).expect("commit transitions");
    let state = engine
        .record_state("p", "source", 2, 2)
        .expect("read state");
    assert!(matches!(
        state.transition,
        Some(Transition {
            status: TransitionStatus::Revoked,
            ordinal: 1,
            ..
        })
    ));
}

#[test]
fn transmit_replay_survives_source_forgetting_at_the_same_tick() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("source", "alice", 1, claim("alice", "item", true))],
            "a1",
            "e1",
            1,
        ))
        .expect("commit source");
    let transmit = TransmitInput {
        on: Default::default(),
        timeline_id: None,
        project_id: "p".to_owned(),
        attempt_id: "transmit-attempt".to_owned(),
        position: Position { tick: 2 },
        from: "alice".to_owned(),
        to: "bob".to_owned(),
        source_record_id: "source".to_owned(),
        record_id: Some("bob-heard".to_owned()),
        attitude: Attitude::Believes,
        confidence: 0.9,
        intent: Intent::Honest,
        distorted_claim: None,
        text: None,
    };
    let original = engine.transmit(transmit.clone()).expect("transmit");
    let mut forgetting = commit(Vec::new(), "forget", "forget-event", 2);
    forgetting.transitions.push(TransitionInput {
        id: Some("forget-transition".to_owned()),
        target_record_id: "source".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick: 2 }),
        provenance: Some(provenance()),
    });
    engine.commit(forgetting).expect("forget source");
    let replay = engine.transmit(transmit).expect("replay transmit");
    assert_eq!(original, replay);
}

#[test]
fn transmit_batch_can_chain_holdings_with_explicit_record_ids() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("source", "alice", 1, claim("alice", "item", true))],
            "a1",
            "e1",
            1,
        ))
        .expect("commit source");
    let outcomes = engine
        .transmit_batch(vec![
            TransmitInput {
                on: Default::default(),
                timeline_id: None,
                project_id: "p".to_owned(),
                attempt_id: "alice-to-bob".to_owned(),
                position: Position { tick: 2 },
                from: "alice".to_owned(),
                to: "bob".to_owned(),
                source_record_id: "source".to_owned(),
                record_id: Some("bob-heard".to_owned()),
                attitude: Attitude::Believes,
                confidence: 0.9,
                intent: Intent::Honest,
                distorted_claim: None,
                text: None,
            },
            TransmitInput {
                on: Default::default(),
                timeline_id: None,
                project_id: "p".to_owned(),
                attempt_id: "bob-to-team".to_owned(),
                position: Position { tick: 3 },
                from: "bob".to_owned(),
                to: "team".to_owned(),
                source_record_id: "bob-heard".to_owned(),
                record_id: Some("team-heard".to_owned()),
                attitude: Attitude::Believes,
                confidence: 0.8,
                intent: Intent::Unknown,
                distorted_claim: None,
                text: None,
            },
        ])
        .expect("transmit batch");
    assert_eq!(outcomes.len(), 2);
    let bob = engine.record("p", "bob-heard").expect("read bob holding");
    let team = engine.record("p", "team-heard").expect("read team holding");
    assert!(
        matches!(bob, Record::Holding { acquisition: Acquisition::ToldBy { source_record_id: Some(source), .. }, .. } if source == "source")
    );
    assert!(
        matches!(team, Record::Holding { acquisition: Acquisition::ToldBy { source_record_id: Some(source), .. }, .. } if source == "bob-heard")
    );
}

#[test]
fn apply_replay_is_idempotent_and_status_changes_atomically() {
    let mut engine = engine();
    let proposal = engine
        .propose(ProposeInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            expected_version: None,
            event: event("proposal-event", 1, Vec::new()),
            records: vec![holding(
                "proposal-record",
                "alice",
                1,
                claim("alice", "item", true),
            )],
            transitions: Vec::new(),
        })
        .expect("create proposal");
    engine
        .review(
            &proposal.id,
            vec![ReviewInput {
                operation_index: 0,
                decision: ReviewDecision::Accepted,
                rationale: "Approved".to_owned(),
            }],
        )
        .expect("review proposal");
    let first = engine
        .apply(&proposal.id, "apply-attempt".to_owned())
        .expect("apply proposal");
    assert_eq!(
        engine.proposal(&proposal.id).expect("read proposal").status,
        ProposalStatus::Applied
    );
    let counts = engine.counts("p").expect("counts after apply");
    let replay = engine
        .apply(&proposal.id, "apply-attempt".to_owned())
        .expect("replay apply");
    assert_eq!(first, replay);
    assert_eq!(counts, engine.counts("p").expect("counts after replay"));

    let invalid = engine
        .propose(ProposeInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            expected_version: None,
            event: event("invalid-proposal-event", 2, Vec::new()),
            records: vec![holding(
                "invalid-proposal-record",
                "missing",
                2,
                claim("alice", "item", true),
            )],
            transitions: Vec::new(),
        })
        .expect("create invalid proposal");
    engine
        .review(
            &invalid.id,
            vec![ReviewInput {
                operation_index: 0,
                decision: ReviewDecision::Accepted,
                rationale: "Try".to_owned(),
            }],
        )
        .expect("review invalid proposal");
    assert!(
        engine
            .apply(&invalid.id, "invalid-apply".to_owned())
            .is_err()
    );
    assert_eq!(
        engine
            .proposal(&invalid.id)
            .expect("read invalid proposal")
            .status,
        ProposalStatus::Pending
    );
    assert!(engine.event("p", "invalid-proposal-event").is_err());
}

#[test]
fn empty_commit_and_apply_without_accepted_operations_fail_without_writes() {
    let mut engine = engine();
    let error = engine
        .commit(commit(Vec::new(), "empty", "empty-event", 1))
        .expect_err("empty commit must fail");
    assert_eq!(error.code, UmweltErrorCode::ValidationFailed);
    assert_eq!(
        error.message,
        "A commit must carry at least one record or transition."
    );
    let proposal = engine
        .propose(ProposeInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            expected_version: None,
            event: event("unaccepted-event", 1, Vec::new()),
            records: vec![holding(
                "unaccepted-record",
                "alice",
                1,
                claim("alice", "item", true),
            )],
            transitions: Vec::new(),
        })
        .expect("create proposal");
    let error = engine
        .apply(&proposal.id, "unaccepted-apply".to_owned())
        .expect_err("apply without acceptance must fail");
    assert_eq!(error.code, UmweltErrorCode::InvalidState);
    assert_eq!(
        engine.proposal(&proposal.id).expect("read proposal").status,
        ProposalStatus::Pending
    );
    assert!(engine.event("p", "unaccepted-event").is_err());
}

#[test]
fn transition_cannot_precede_its_target_record() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding(
                "future-source",
                "alice",
                10,
                claim("alice", "item", true),
            )],
            "a1",
            "e1",
            10,
        ))
        .expect("commit target");
    let mut input = commit(Vec::new(), "a2", "e2", 5);
    input.transitions.push(TransitionInput {
        id: Some("too-early".to_owned()),
        target_record_id: "future-source".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick: 5 }),
        provenance: Some(provenance()),
    });
    assert!(engine.commit(input).is_err());
}

#[test]
fn attempt_id_reuse_with_a_different_payload_conflicts_without_writes() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("first", "alice", 1, claim("alice", "item", true))],
            "same-attempt",
            "e1",
            1,
        ))
        .expect("first commit");
    let counts = engine.counts("p").expect("counts before conflict");
    let error = engine
        .commit(commit(
            vec![holding("second", "bob", 2, claim("bob", "item", true))],
            "same-attempt",
            "e2",
            2,
        ))
        .expect_err("payload mismatch must conflict");
    assert_eq!(error.code, UmweltErrorCode::Conflict);
    assert_eq!(
        error.message,
        "attemptId was already used with a different payload."
    );
    assert_eq!(error.details, Some(json!({ "attemptId": "same-attempt" })));
    assert_eq!(counts, engine.counts("p").expect("counts after conflict"));
}

#[test]
fn witnessed_event_and_entity_fields_are_validated() {
    let mut engine = engine();
    let witnessed = RecordInput::Holding {
        common: common("witnessed", 1),
        holder: "alice".to_owned(),
        attitude: Attitude::Believes,
        claim: claim("alice", "item", true),
        acquisition: Acquisition::Witnessed {
            event_id: "missing-event".to_owned(),
        },
        confidence: 1.0,
        accessibility: Accessibility::Available,
        text: None,
        secrecy: None,
    };
    assert!(
        engine
            .commit(commit(vec![witnessed], "a1", "e1", 1))
            .is_err()
    );

    engine
        .put_entity_type(EntityType {
            id: "typed".to_owned(),
            project_id: "p".to_owned(),
            name: "Typed".to_owned(),
            kind: EntityKind::Other,
            fields: vec![FieldDef {
                id: "known-field".to_owned(),
                key: "known".to_owned(),
                label: "Known".to_owned(),
                field_type: FieldType::Text,
                description: None,
                constraints: None,
            }],
        })
        .expect("create typed entity type");
    let invalid = Entity {
        id: "invalid-entity".to_owned(),
        project_id: "p".to_owned(),
        type_id: "typed".to_owned(),
        name: "Invalid".to_owned(),
        aliases: Vec::new(),
        fields: serde_json::Map::from_iter([("unknown".to_owned(), json!(true))]),
        external_ref: None,
        created_at: Position { tick: 0 },
        archived: false,
    };
    assert!(engine.put_entity(invalid).is_err());
}

#[test]
fn indexed_claim_reads_apply_filters_and_bitemporal_windows() {
    let mut engine = engine();
    engine
        .commit(commit(
            vec![holding("indexed", "alice", 4, claim("alice", "item", true))],
            "indexed-attempt",
            "indexed-event",
            4,
        ))
        .expect("commit indexed holding");
    let query = RecordQuery {
        axis: None,
        project_id: "p".to_owned(),
        timeline_id: MAIN_TIMELINE_ID.to_owned(),
        ancestors: Vec::new(),
        holder_id: Some("alice".to_owned()),
        record_kind: None,
        claim_key: None,
        slot_key: None,
        subject_id: Some("alice".to_owned()),
        predicate: Some("related".to_owned()),
        object_entity_id: Some("item".to_owned()),
        about_entity_id: None,
        attitudes: vec![Attitude::Believes],
        minimum_confidence: Some(0.9),
        include_withheld: false,
        tick: 4,
        as_of_version: 1,
        limit: 10,
        cursor: None,
    };
    let page = engine.query_records(&query).expect("query indexed records");
    assert_eq!(page.records.len(), 1);
    assert!(page.next_cursor.is_none());
    let mut earlier = query;
    earlier.tick = 3;
    assert!(
        engine
            .query_records(&earlier)
            .expect("query earlier window")
            .records
            .is_empty()
    );
}

#[test]
fn rejected_proposal_persists_and_returns_its_rationale() {
    let mut engine = engine();
    let proposal = engine
        .propose(ProposeInput {
            timeline_id: None,
            project_id: "p".to_owned(),
            expected_version: None,
            event: event("rejected-event", 1, Vec::new()),
            records: vec![holding(
                "rejected-record",
                "alice",
                1,
                claim("alice", "item", true),
            )],
            transitions: Vec::new(),
        })
        .expect("create proposal");
    let rejected = engine
        .reject(&proposal.id, "The evidence is insufficient.")
        .expect("reject proposal");
    assert_eq!(rejected.status, ProposalStatus::Rejected);
    assert_eq!(
        rejected.rejection_rationale.as_deref(),
        Some("The evidence is insufficient.")
    );
    assert_eq!(
        engine
            .proposal(&proposal.id)
            .expect("reload rejected proposal")
            .rejection_rationale
            .as_deref(),
        Some("The evidence is insufficient.")
    );
}

#[test]
fn indexed_claim_lookup_remains_correct_with_twenty_thousand_holdings() {
    let mut engine = engine();
    for index in 0..40 {
        let id = format!("holder-{index:02}");
        engine
            .put_entity(Entity {
                id: id.clone(),
                project_id: "p".to_owned(),
                type_id: "agent".to_owned(),
                name: id,
                aliases: Vec::new(),
                fields: serde_json::Map::new(),
                external_ref: None,
                created_at: Position { tick: 0 },
                archived: false,
            })
            .expect("create scale-test holder");
    }
    let target_index = 12_345;
    let mut target_key = None;
    let records = (0..20_000)
        .map(|index| {
            let mut value = claim("alice", "item", true);
            value.qualifiers.insert("sequence".to_owned(), json!(index));
            if index == target_index {
                target_key = Some(calculate_claim_key(&value));
            }
            holding(
                &format!("scale-record-{index:05}"),
                &format!("holder-{:02}", index % 40),
                10,
                value,
            )
        })
        .collect();
    engine
        .commit(commit(records, "scale-attempt", "scale-event", 10))
        .expect("commit scale-test holdings");
    let target_key = target_key.expect("capture target claim key");
    let base = RecordQuery {
        axis: None,
        project_id: "p".to_owned(),
        timeline_id: MAIN_TIMELINE_ID.to_owned(),
        ancestors: Vec::new(),
        holder_id: Some(format!("holder-{:02}", target_index % 40)),
        record_kind: None,
        claim_key: Some(target_key.clone()),
        slot_key: None,
        subject_id: None,
        predicate: None,
        object_entity_id: None,
        about_entity_id: None,
        attitudes: Vec::new(),
        minimum_confidence: None,
        include_withheld: false,
        tick: 10,
        as_of_version: 1,
        limit: 10,
        cursor: None,
    };
    let by_holder_and_key = engine
        .query_records(&base)
        .expect("query holder and claim key");
    assert_eq!(by_holder_and_key.records.len(), 1);
    let mut by_key = base;
    by_key.holder_id = None;
    let by_key = engine
        .query_records(&by_key)
        .expect("query claim key across holders");
    assert_eq!(by_key.records.len(), 1);
}
