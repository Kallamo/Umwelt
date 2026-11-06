// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;

use serde_json::{Map, json};
use umwelt_core::domain::*;

fn minimal_common() -> RecordCommon {
    RecordCommon {
        id: "r1".to_owned(),
        project_id: "p".to_owned(),
        timeline_id: MAIN_TIMELINE_ID.to_owned(),
        position: Position { tick: 3 },
        recorded_at: 1,
        event_id: "e1".to_owned(),
        provenance: Provenance {
            origin: Origin::Human { actor_id: None },
            reason: None,
            metadata: None,
        },
    }
}

fn gift_claim() -> Claim {
    Claim {
        subject: "player".to_owned(),
        predicate: "gave-gift".to_owned(),
        object: ClaimObject::Entity {
            id: "mira".to_owned(),
        },
        qualifiers: BTreeMap::from([("item".to_owned(), json!("lantern"))]),
        polarity: true,
        about: None,
    }
}

#[test]
fn complete_holding_has_the_exact_wire_format() {
    let record = Record::Holding {
        common: minimal_common(),
        holder: "noah".to_owned(),
        attitude: Attitude::Believes,
        claim: gift_claim(),
        acquisition: Acquisition::Witnessed {
            event_id: "e1".to_owned(),
        },
        confidence: 1.0,
        accessibility: Accessibility::Available,
        text: None,
        secrecy: None,
    };

    assert_eq!(
        serde_json::to_string(&record).expect("serialize holding"),
        r#"{"kind":"holding","id":"r1","projectId":"p","timelineId":"main","position":{"tick":3},"recordedAt":1,"eventId":"e1","provenance":{"origin":{"kind":"human"}},"holder":"noah","attitude":"believes","claim":{"subject":"player","predicate":"gave-gift","object":{"kind":"entity","id":"mira"},"qualifiers":{"item":"lantern"},"polarity":true},"acquisition":{"kind":"witnessed","eventId":"e1"},"confidence":1.0,"accessibility":"available"}"#
    );
}

#[test]
fn entity_type_and_field_definition_have_the_exact_wire_format() {
    let entity_type = EntityType {
        id: "person".to_owned(),
        project_id: "p".to_owned(),
        name: "Person".to_owned(),
        kind: EntityKind::Agent,
        fields: vec![FieldDef {
            id: "friend".to_owned(),
            key: "friend".to_owned(),
            label: "Friend".to_owned(),
            field_type: FieldType::EntityReference,
            description: None,
            constraints: None,
        }],
    };

    assert_eq!(
        serde_json::to_string(&entity_type).expect("serialize entity type"),
        r#"{"id":"person","projectId":"p","name":"Person","kind":"agent","fields":[{"id":"friend","key":"friend","label":"Friend","type":"entity-reference"}]}"#
    );
}

#[test]
fn record_without_optional_fields_omits_nulls() {
    let record = Record::Fact {
        common: RecordCommon {
            provenance: Provenance {
                origin: Origin::Import,
                reason: None,
                metadata: None,
            },
            ..minimal_common()
        },
        claim: Claim {
            subject: "player".to_owned(),
            predicate: "waited".to_owned(),
            object: ClaimObject::None,
            qualifiers: BTreeMap::new(),
            polarity: true,
            about: None,
        },
    };

    let serialized = serde_json::to_string(&record).expect("serialize fact");
    assert_eq!(
        serialized,
        r#"{"kind":"fact","id":"r1","projectId":"p","timelineId":"main","position":{"tick":3},"recordedAt":1,"eventId":"e1","provenance":{"origin":{"kind":"import"}},"claim":{"subject":"player","predicate":"waited","object":{"kind":"none"},"qualifiers":{},"polarity":true}}"#
    );
    assert!(!serialized.contains("null"));
}

#[test]
fn empty_collections_default_during_deserialization() {
    let entity: Entity = serde_json::from_value(json!({
        "id": "e1",
        "projectId": "p",
        "typeId": "person",
        "name": "Mara",
        "createdAt": { "tick": 0 },
        "archived": false
    }))
    .expect("deserialize entity with default collections");
    assert!(entity.aliases.is_empty());
    assert_eq!(entity.fields, Map::new());

    let event: Event = serde_json::from_value(json!({
        "id": "event-1",
        "projectId": "p",
        "timelineId": "main",
        "type": "arrival",
        "title": "Arrival",
        "position": { "tick": 0 },
        "provenance": { "origin": { "kind": "import" } },
        "recordedAt": 1
    }))
    .expect("deserialize event with default collections");
    assert!(event.participants.is_empty());
    assert!(event.evidence.is_empty());
}
