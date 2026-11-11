// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, json};
use umwelt_core::domain::*;

fn round_trip<T>(value: &T)
where
    T: std::fmt::Debug + PartialEq + Serialize + DeserializeOwned,
{
    let encoded = serde_json::to_string(value).expect("serialize contract value");
    let decoded = serde_json::from_str::<T>(&encoded).expect("deserialize contract value");
    assert_eq!(&decoded, value);
}

fn position() -> Position {
    Position { tick: 12 }
}

fn provenance() -> Provenance {
    Provenance {
        origin: Origin::Human {
            actor_id: Some("author".to_owned()),
        },
        reason: Some("test".to_owned()),
        metadata: Some(Map::from_iter([("source".to_owned(), json!("suite"))])),
    }
}

fn common() -> RecordCommon {
    RecordCommon {
        id: "record-1".to_owned(),
        project_id: "project-1".to_owned(),
        timeline_id: MAIN_TIMELINE_ID.to_owned(),
        position: position(),
        recorded_at: 3,
        event_id: "event-1".to_owned(),
        provenance: provenance(),
    }
}

fn claim() -> Claim {
    Claim {
        subject: "mara".to_owned(),
        predicate: "knows-route".to_owned(),
        object: ClaimObject::Value { value: json!(true) },
        qualifiers: BTreeMap::from([("region".to_owned(), json!("north"))]),
        polarity: true,
        about: Some("route-1".to_owned()),
    }
}

#[test]
fn time_project_and_entity_types_round_trip() {
    round_trip(&position());
    for value in [VocabularyMode::Strict, VocabularyMode::Open] {
        round_trip(&value);
    }
    round_trip(&Project {
        id: "project-1".to_owned(),
        name: "Story".to_owned(),
        description: Some("A test project".to_owned()),
        timescale: "scene".to_owned(),
        vocabulary_mode: VocabularyMode::Strict,
        created_at: "2026-09-28T12:00:00Z".to_owned(),
        version: 4,
    });

    for value in [EntityKind::Agent, EntityKind::Group, EntityKind::Other] {
        round_trip(&value);
    }
    for value in [
        FieldType::Text,
        FieldType::Number,
        FieldType::Boolean,
        FieldType::Date,
        FieldType::Enum,
        FieldType::EntityReference,
        FieldType::EntityReferenceList,
        FieldType::Json,
    ] {
        round_trip(&value);
    }
    let option = FieldOption {
        value: "red".to_owned(),
        label: "Red".to_owned(),
    };
    round_trip(&option);
    let constraints = FieldConstraints {
        required: Some(true),
        minimum: Some(0.0),
        maximum: Some(10.0),
        min_length: Some(1),
        max_length: Some(20),
        options: Some(vec![option]),
        allowed_entity_type_ids: Some(vec!["person".to_owned()]),
    };
    round_trip(&constraints);
    let field = FieldDef {
        id: "field-1".to_owned(),
        key: "color".to_owned(),
        label: "Color".to_owned(),
        field_type: FieldType::Enum,
        description: Some("Favorite color".to_owned()),
        constraints: Some(constraints),
    };
    round_trip(&field);
    round_trip(&EntityType {
        id: "person".to_owned(),
        project_id: "project-1".to_owned(),
        name: "Person".to_owned(),
        kind: EntityKind::Agent,
        fields: vec![field],
    });
    let external = ExternalRef {
        system: "game".to_owned(),
        id: "npc-7".to_owned(),
    };
    round_trip(&external);
    round_trip(&Entity {
        id: "mara".to_owned(),
        project_id: "project-1".to_owned(),
        type_id: "person".to_owned(),
        name: "Mara".to_owned(),
        aliases: vec!["The Guide".to_owned()],
        fields: Map::from_iter([("color".to_owned(), json!("red"))]),
        external_ref: Some(external),
        created_at: position(),
        archived: false,
    });
}

#[test]
fn predicate_and_claim_types_round_trip() {
    for value in [ObjectKind::Entity, ObjectKind::Value, ObjectKind::None] {
        round_trip(&value);
    }
    for value in [Cardinality::One, Cardinality::Many] {
        round_trip(&value);
    }
    round_trip(&PredicateDef {
        id: "gave-gift".to_owned(),
        label: "Gave gift".to_owned(),
        object: ObjectKind::Entity,
        cardinality: Cardinality::Many,
        subject_types: Some(vec!["person".to_owned()]),
    });
    for value in [
        ClaimObject::Entity {
            id: "mira".to_owned(),
        },
        ClaimObject::Value {
            value: json!({"item": "lantern"}),
        },
        ClaimObject::None,
    ] {
        round_trip(&value);
    }
    round_trip(&claim());
    for predicate in reserved_predicates() {
        round_trip(&predicate);
    }
}

#[test]
fn provenance_and_acquisition_variants_round_trip() {
    for value in [
        Attitude::Knows,
        Attitude::Believes,
        Attitude::Suspects,
        Attitude::Remembers,
    ] {
        round_trip(&value);
    }
    for value in [
        Accessibility::Available,
        Accessibility::Suppressed,
        Accessibility::Inaccessible,
    ] {
        round_trip(&value);
    }
    for value in [Intent::Honest, Intent::Deceptive, Intent::Unknown] {
        round_trip(&value);
    }
    for value in [
        Acquisition::Witnessed {
            event_id: "event-1".to_owned(),
        },
        Acquisition::ToldBy {
            source_holder_id: "mara".to_owned(),
            source_record_id: Some("record-0".to_owned()),
            intent: Intent::Honest,
        },
        Acquisition::Evidence {
            evidence_id: "evidence-1".to_owned(),
        },
        Acquisition::Inferred {
            from_record_ids: vec!["record-0".to_owned()],
            rule: Some("modus-ponens".to_owned()),
        },
        Acquisition::Authored,
    ] {
        round_trip(&value);
    }
    for value in [
        Origin::Human {
            actor_id: Some("author".to_owned()),
        },
        Origin::Model {
            model: "model-1".to_owned(),
            prompt_hash: Some("abc".to_owned()),
        },
        Origin::System {
            name: "importer".to_owned(),
        },
        Origin::Import,
    ] {
        round_trip(&value);
    }
    round_trip(&provenance());
    round_trip(&common());
}

#[test]
fn every_record_variant_round_trips_with_kind_tag() {
    let records = [
        Record::Fact {
            common: common(),
            claim: claim(),
        },
        Record::Holding {
            common: common(),
            holder: "mara".to_owned(),
            attitude: Attitude::Knows,
            claim: claim(),
            acquisition: Acquisition::Witnessed {
                event_id: "event-1".to_owned(),
            },
            confidence: 0.9,
            accessibility: Accessibility::Available,
            text: Some("Mara saw it happen.".to_owned()),
            secrecy: None,
        },
        Record::Awareness {
            common: common(),
            holder: "mara".to_owned(),
            target: "mira".to_owned(),
        },
        Record::Disposition {
            common: common(),
            holder: "mara".to_owned(),
            target: "mira".to_owned(),
            dimension: "trust".to_owned(),
            value: json!(0.75),
        },
    ];
    for record in records {
        let encoded = serde_json::to_value(&record).expect("serialize record");
        assert!(encoded.get("kind").is_some());
        round_trip(&record);
    }
}

#[test]
fn transition_event_and_evidence_types_round_trip() {
    for value in [
        TransitionStatus::Retracted,
        TransitionStatus::Revoked,
        TransitionStatus::Forgotten,
        TransitionStatus::Abandoned,
        TransitionStatus::Superseded,
    ] {
        round_trip(&value);
    }
    round_trip(&Transition {
        id: "transition-1".to_owned(),
        project_id: "project-1".to_owned(),
        timeline_id: MAIN_TIMELINE_ID.to_owned(),
        target_record_id: "record-1".to_owned(),
        status: TransitionStatus::Superseded,
        superseded_by: Some("record-2".to_owned()),
        position: position(),
        recorded_at: 4,
        ordinal: 0,
        event_id: "event-1".to_owned(),
        provenance: provenance(),
    });
    let evidence_ref = EvidenceRef {
        evidence_source_id: "evidence-1".to_owned(),
        locator: Some("chapter-2".to_owned()),
        excerpt: Some("A relevant passage".to_owned()),
        metadata: Some(Map::new()),
    };
    round_trip(&evidence_ref);
    round_trip(&EvidenceSource {
        id: "evidence-1".to_owned(),
        project_id: "project-1".to_owned(),
        source_type: "document".to_owned(),
        title: "Notes".to_owned(),
        description: None,
        locator: Some("notes.md".to_owned()),
        content_hash: Some("sha256:abc".to_owned()),
        metadata: Some(Map::new()),
        created_at: "2026-09-28T12:00:00Z".to_owned(),
    });
    round_trip(&Event {
        on: Default::default(),
        id: "event-1".to_owned(),
        project_id: "project-1".to_owned(),
        timeline_id: MAIN_TIMELINE_ID.to_owned(),
        event_type: "gift".to_owned(),
        title: "A gift was given".to_owned(),
        description: None,
        position: position(),
        participants: vec!["player".to_owned(), "mira".to_owned()],
        location: Some("farm".to_owned()),
        evidence: vec![evidence_ref],
        provenance: provenance(),
        recorded_at: 3,
    });
}

#[test]
fn field_names_are_camel_case_and_enum_tags_are_kebab_case() {
    let value = serde_json::to_value(Project {
        id: "project-1".to_owned(),
        name: "Story".to_owned(),
        description: None,
        timescale: "scene".to_owned(),
        vocabulary_mode: VocabularyMode::Open,
        created_at: "2026-09-28T12:00:00Z".to_owned(),
        version: 0,
    })
    .expect("serialize project");
    assert!(value.get("vocabularyMode").is_some());
    assert_eq!(value["vocabularyMode"], "open");

    let acquisition = serde_json::to_value(Acquisition::ToldBy {
        source_holder_id: "mara".to_owned(),
        source_record_id: None,
        intent: Intent::Unknown,
    })
    .expect("serialize acquisition");
    assert_eq!(acquisition["kind"], "told-by");
    assert!(acquisition.get("sourceHolderId").is_some());
    assert_eq!(acquisition["intent"], "unknown");
    assert!(acquisition.get("sourceRecordId").is_none());
}

#[test]
fn position_and_entity_field_validation_are_preserved() {
    assert!(Position { tick: 0 }.validate().is_ok());
    assert!(Position { tick: -1 }.validate().is_err());

    let entity_type = EntityType {
        id: "person".to_owned(),
        project_id: "project-1".to_owned(),
        name: "Person".to_owned(),
        kind: EntityKind::Agent,
        fields: vec![FieldDef {
            id: "field-1".to_owned(),
            key: "name".to_owned(),
            label: "Name".to_owned(),
            field_type: FieldType::Text,
            description: None,
            constraints: Some(FieldConstraints {
                required: Some(true),
                ..FieldConstraints::default()
            }),
        }],
    };
    assert!(
        validate_entity_fields(
            &Map::from_iter([("name".to_owned(), json!("Mara"))]),
            &entity_type
        )
        .is_ok()
    );
    assert!(validate_entity_fields(&Map::new(), &entity_type).is_err());
    assert!(
        validate_entity_fields(
            &Map::from_iter([("unknown".to_owned(), json!(true))]),
            &entity_type
        )
        .is_err()
    );
}

#[test]
fn write_contract_types_round_trip() {
    let event = EventInput {
        on: Default::default(),
        id: Some("event-1".to_owned()),
        event_type: "test".to_owned(),
        title: "Test".to_owned(),
        description: None,
        position: position(),
        participants: vec!["mara".to_owned()],
        location: None,
        evidence: Vec::new(),
        provenance: provenance(),
    };
    let record = RecordInput::Holding {
        common: RecordInputCommon {
            id: Some("record-1".to_owned()),
            position: Some(position()),
            provenance: Some(provenance()),
        },
        holder: "mara".to_owned(),
        attitude: Attitude::Believes,
        claim: claim(),
        acquisition: Acquisition::Authored,
        confidence: 0.8,
        accessibility: Accessibility::Available,
        text: None,
        secrecy: None,
    };
    let transition = TransitionInput {
        id: Some("transition-1".to_owned()),
        target_record_id: "record-1".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(position()),
        provenance: Some(provenance()),
    };
    let commit = CommitInput {
        timeline_id: None,
        project_id: "project-1".to_owned(),
        attempt_id: "attempt-1".to_owned(),
        expected_version: Some(2),
        event: event.clone(),
        records: vec![record.clone()],
        transitions: vec![transition.clone()],
        preconditions: vec![Precondition::ActiveHolding {
            record_id: "record-0".to_owned(),
            holder: "mara".to_owned(),
            at: position(),
            copy_claim_to_operation: None,
        }],
    };
    round_trip(&event);
    round_trip(record.common());
    round_trip(&record);
    round_trip(&transition);
    round_trip(&commit);
    round_trip(&ProposeInput {
        timeline_id: None,
        project_id: commit.project_id.clone(),
        expected_version: commit.expected_version,
        event: commit.event.clone(),
        records: commit.records.clone(),
        transitions: commit.transitions.clone(),
    });
    round_trip(&Warning {
        code: "UNKNOWN_PREDICATE".to_owned(),
        message: "Unknown predicate".to_owned(),
    });
    round_trip(&CommitOutcome {
        event_id: "event-1".to_owned(),
        record_ids: vec!["record-1".to_owned()],
        transition_ids: vec!["transition-1".to_owned()],
        version: 3,
        warnings: Vec::new(),
    });
    for status in [
        ProposalStatus::Pending,
        ProposalStatus::Applied,
        ProposalStatus::PartiallyApplied,
        ProposalStatus::Rejected,
    ] {
        round_trip(&status);
    }
    for decision in [ReviewDecision::Accepted, ReviewDecision::Rejected] {
        round_trip(&decision);
    }
    let review = ProposalReview {
        id: "review-1".to_owned(),
        operation_index: 0,
        decision: ReviewDecision::Accepted,
        rationale: "Approved".to_owned(),
        created_at: "2026-09-28T12:00:00Z".to_owned(),
    };
    round_trip(&ReviewInput {
        operation_index: review.operation_index,
        decision: review.decision,
        rationale: review.rationale.clone(),
    });
    round_trip(&review);
    round_trip(&Proposal {
        id: "proposal-1".to_owned(),
        status: ProposalStatus::Rejected,
        rejection_rationale: Some("The evidence is insufficient.".to_owned()),
        input: ProposeInput {
            timeline_id: None,
            project_id: commit.project_id,
            expected_version: commit.expected_version,
            event: commit.event,
            records: commit.records,
            transitions: commit.transitions,
        },
        reviews: vec![review],
    });
    round_trip(&TransmitInput {
        on: Default::default(),
        timeline_id: None,
        project_id: "project-1".to_owned(),
        attempt_id: "attempt-2".to_owned(),
        position: position(),
        from: "mara".to_owned(),
        to: "mira".to_owned(),
        source_record_id: "record-1".to_owned(),
        record_id: Some("record-2".to_owned()),
        attitude: Attitude::Believes,
        confidence: 0.7,
        intent: Intent::Honest,
        distorted_claim: None,
        text: None,
    });
    round_trip(&RecordState {
        record: Record::Fact {
            common: common(),
            claim: claim(),
        },
        transition: None,
    });
}

#[test]
fn read_contract_types_round_trip() {
    round_trip(&At {
        axis: None,
        tick: Some(12),
        as_of_version: Some(3),
    });
    round_trip(&Page {
        limit: 25,
        cursor: Some("opaque".to_owned()),
    });
    round_trip(&ClaimRef::Claim { claim: claim() });
    round_trip(&HoldingFilter {
        about: Some("mara".to_owned()),
        predicate: Some("knows".to_owned()),
        attitude: vec![Attitude::Believes],
        min_confidence: Some(0.5),
        include_withheld: true,
    });
    round_trip(&HoldsResult {
        holds: true,
        holds_as: Some(HoldsAs::Same),
        attitude: Some(Attitude::Believes),
        polarity: Some(true),
        confidence: Some(0.8),
        record_id: Some("record-1".to_owned()),
    });
    round_trip(&PageResult {
        items: vec![Record::Fact {
            common: common(),
            claim: claim(),
        }],
        next_cursor: Some("opaque".to_owned()),
    });
    round_trip(&Contradiction {
        holding_id: "holding-1".to_owned(),
        kind: ContradictionKind::VsCanon,
        other_record_id: "fact-1".to_owned(),
        reason: ContradictionReason::OppositePolarity,
    });
    round_trip(&SearchFilter {
        holder: Some("mara".to_owned()),
        doc_type: Some("record".to_owned()),
    });
}

#[test]
fn internal_transmit_precondition_field_is_ignored_on_the_wire() {
    let input: CommitInput = serde_json::from_value(json!({
        "projectId": "project-1",
        "attemptId": "attempt-1",
        "event": {
            "id": "event-1",
            "type": "test",
            "title": "Test",
            "position": { "tick": 1 },
            "participants": [],
            "evidence": [],
            "provenance": { "origin": { "kind": "system", "name": "test" } }
        },
        "records": [],
        "transitions": [],
        "preconditions": [{
            "kind": "active-holding",
            "recordId": "record-1",
            "holder": "mara",
            "at": { "tick": 1 },
            "copyClaimToOperation": 0
        }]
    }))
    .expect("deserialize commit input");
    assert!(matches!(
        input.preconditions.as_slice(),
        [Precondition::ActiveHolding {
            copy_claim_to_operation: None,
            ..
        }]
    ));
    let serialized = serde_json::to_string(&input).expect("serialize commit input");
    assert!(!serialized.contains("copyClaimToOperation"));
}
