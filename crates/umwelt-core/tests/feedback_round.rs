// SPDX-License-Identifier: AGPL-3.0-only
use serde_json::{Value, json};
use umwelt_core::domain::*;
use umwelt_core::{UmweltErrorCode, request_fingerprint};

fn claim(polarity: bool) -> Claim {
    serde_json::from_value(json!({
        "subject": "bob",
        "predicate": "stole",
        "object": { "kind": "entity", "id": "necklace" },
        "polarity": polarity
    }))
    .expect("claim")
}

fn shared(value: Value) -> TransmitShared {
    serde_json::from_value(value).expect("shared")
}

fn item(value: Value) -> TransmitBatchItem {
    serde_json::from_value(value).expect("item")
}

#[test]
fn the_keys_of_a_claim_ignore_its_polarity() {
    let affirmed = ClaimKeyResult::of(&claim(true));
    let denied = ClaimKeyResult::of(&claim(false));
    assert_eq!(affirmed, denied);
    assert_eq!(affirmed.claim_key.len(), 64);
    assert_ne!(affirmed.claim_key, affirmed.slot_key);
}

#[test]
fn an_item_wins_over_shared_and_takes_the_rest_from_it() {
    let common = shared(json!({
        "projectId": "p", "from": "sera", "sourceRecordId": "secret",
        "intent": "honest", "confidence": 0.7, "position": { "tick": 2 }
    }));
    let plain = item(json!({ "attemptId": "a", "to": "orrin" }))
        .complete(&common)
        .expect("complete");
    assert_eq!(plain.project_id, "p");
    assert_eq!(plain.from, "sera");
    assert_eq!(plain.to, "orrin");
    assert_eq!(plain.confidence, 0.7);
    assert_eq!(plain.intent, Intent::Honest);
    assert_eq!(plain.attitude, Attitude::Believes);
    assert_eq!(plain.position, Position { tick: 2 });

    let own = item(json!({
        "attemptId": "b", "to": "vane", "from": "tilde", "confidence": 0.2,
        "intent": "deceptive", "attitude": "suspects", "position": { "tick": 4 }
    }))
    .complete(&common)
    .expect("complete");
    assert_eq!(own.from, "tilde");
    assert_eq!(own.confidence, 0.2);
    assert_eq!(own.intent, Intent::Deceptive);
    assert_eq!(own.attitude, Attitude::Suspects);
    assert_eq!(own.position, Position { tick: 4 });
}

#[test]
fn nothing_is_assumed_for_what_neither_place_gives() {
    let common = shared(json!({ "projectId": "p", "from": "sera", "sourceRecordId": "s" }));
    for (missing, value) in [
        (
            "confidence",
            json!({ "attemptId": "a", "to": "o", "intent": "honest", "position": { "tick": 1 } }),
        ),
        (
            "intent",
            json!({ "attemptId": "a", "to": "o", "confidence": 1, "position": { "tick": 1 } }),
        ),
        (
            "position",
            json!({ "attemptId": "a", "to": "o", "confidence": 1, "intent": "honest" }),
        ),
        (
            "to",
            json!({ "attemptId": "a", "confidence": 1, "intent": "honest", "position": { "tick": 1 } }),
        ),
    ] {
        let error = item(value).complete(&common).expect_err(missing);
        assert_eq!(error.code, UmweltErrorCode::ValidationFailed);
        assert!(error.message.contains(missing), "{}", error.message);
    }
}

#[test]
fn shared_cannot_carry_what_identifies_one_transmit() {
    for field in ["attemptId", "recordId", "text", "distortedClaim"] {
        let mut value = json!({ "projectId": "p" });
        value[field] = json!("x");
        assert!(
            serde_json::from_value::<TransmitShared>(value).is_err(),
            "{field} must not be shareable"
        );
    }
}

#[test]
fn a_transmit_is_the_same_attempt_wherever_its_fields_are_written() {
    let common = shared(json!({
        "projectId": "p", "from": "sera", "sourceRecordId": "s",
        "intent": "honest", "confidence": 0.7, "position": { "tick": 2 }
    }));
    let from_shared = item(json!({ "attemptId": "a", "to": "orrin" }))
        .complete(&common)
        .expect("complete");
    let written_out = item(json!({
        "attemptId": "a", "to": "orrin", "projectId": "p", "from": "sera",
        "sourceRecordId": "s", "intent": "honest", "confidence": 0.7, "position": { "tick": 2 }
    }))
    .complete(&TransmitShared::default())
    .expect("complete");
    assert_eq!(from_shared, written_out);
    assert_eq!(
        request_fingerprint("transmit", &from_shared).expect("fingerprint"),
        request_fingerprint("transmit", &written_out).expect("fingerprint")
    );
    // A complete TransmitInput is also a batch item.
    let again = TransmitBatchItem::from(written_out.clone())
        .complete(&TransmitShared::default())
        .expect("complete");
    assert_eq!(again, written_out);
}

#[test]
fn the_holder_reads_an_unknown_intent() {
    let mut told: Acquisition = serde_json::from_value(json!({
        "kind": "told-by", "sourceHolderId": "felix", "sourceRecordId": "r", "intent": "deceptive"
    }))
    .expect("acquisition");
    told.hide_intent();
    assert_eq!(
        serde_json::to_value(&told).expect("json"),
        json!({ "kind": "told-by", "sourceHolderId": "felix", "sourceRecordId": "r", "intent": "unknown" })
    );
    let mut seen: Acquisition =
        serde_json::from_value(json!({ "kind": "witnessed", "eventId": "e" }))
            .expect("acquisition");
    let before = seen.clone();
    seen.hide_intent();
    assert_eq!(seen, before);
}

fn manifest(value: Value) -> EnsureWorldInput {
    serde_json::from_value(value).expect("manifest")
}

fn ball() -> EnsureWorldInput {
    manifest(json!({
        "project": { "id": "ball", "name": "The Ball", "timescale": "scene" },
        "entityTypes": [{ "id": "person", "name": "Person", "kind": "agent" }],
        "entities": [{ "id": "ana", "typeId": "person", "name": "Ana" }],
        "predicates": [{ "id": "stole", "label": "stole", "object": "entity", "cardinality": "many" }],
        "axes": [{ "id": "reading", "label": "Reading order" }]
    }))
}

/// The stored world right after a manifest was applied to an empty file.
fn applied(input: &EnsureWorldInput) -> StoredWorld {
    let (write, divergences) = plan_world(input, &StoredWorld::default(), "now").expect("plan");
    assert!(divergences.is_empty());
    StoredWorld {
        project: write.project,
        entity_types: write.entity_types,
        entities: write.entities,
        predicates: write.predicates,
        axes: write.axes,
    }
}

#[test]
fn an_empty_world_gets_everything_and_a_complete_one_nothing() {
    let input = ball();
    let (write, divergences) = plan_world(&input, &StoredWorld::default(), "now").expect("plan");
    assert!(divergences.is_empty());
    assert_eq!(write.project.as_ref().map(|p| p.version), Some(0));
    assert_eq!(write.project_id, "ball");
    assert_eq!(
        (
            write.entity_types.len(),
            write.entities.len(),
            write.predicates.len(),
            write.axes.len()
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(write.entities[0].project_id, "ball");

    let (again, divergences) = plan_world(&input, &applied(&input), "later").expect("plan");
    assert!(divergences.is_empty());
    assert!(again.project.is_none());
    assert!(again.entity_types.is_empty() && again.entities.is_empty());
    assert!(again.predicates.is_empty() && again.axes.is_empty());
}

#[test]
fn an_entity_may_change_in_play_but_a_definition_may_not() {
    let input = ball();
    let mut stored = applied(&input);
    stored.entities[0].name = "Ana of the Vale".to_owned();
    stored.entities[0].aliases = vec!["the witness".to_owned()];
    let (_, divergences) = plan_world(&input, &stored, "now").expect("plan");
    assert!(divergences.is_empty(), "{divergences:?}");

    stored.entities[0].type_id = "thing".to_owned();
    stored.predicates[0].cardinality = Cardinality::One;
    stored.axes[0].label = "Pages".to_owned();
    stored.entity_types[0].kind = EntityKind::Other;
    let (write, divergences) = plan_world(&input, &stored, "now").expect("plan");
    assert!(write.entities.is_empty());
    let found = divergences
        .iter()
        .map(|d| format!("{}:{}:{}", d.kind, d.id, d.field))
        .collect::<Vec<_>>();
    assert_eq!(
        found,
        [
            "entityType:person:kind",
            "entity:ana:typeId",
            "predicate:stole:cardinality",
            "axis:reading:label"
        ]
    );
}

#[test]
fn a_manifest_is_checked_before_anything_is_compared() {
    let twice = manifest(json!({
        "project": { "id": "ball", "name": "The Ball", "timescale": "scene" },
        "entities": [
            { "id": "ana", "typeId": "person", "name": "Ana" },
            { "id": "ana", "typeId": "person", "name": "Ana" }
        ]
    }));
    let reserved = manifest(json!({
        "project": { "id": "ball", "name": "The Ball", "timescale": "scene" },
        "predicates": [{ "id": "umwelt:mine", "label": "mine", "object": "none", "cardinality": "one" }]
    }));
    let no_id =
        manifest(json!({ "project": { "id": " ", "name": "The Ball", "timescale": "scene" } }));
    let tick_axis = manifest(json!({
        "project": { "id": "ball", "name": "The Ball", "timescale": "scene" },
        "axes": [{ "id": "tick", "label": "Story" }]
    }));
    for input in [twice, reserved, no_id, tick_axis] {
        let error = plan_world(&input, &StoredWorld::default(), "now").expect_err("refused");
        assert_eq!(error.code, UmweltErrorCode::ValidationFailed);
    }
}
