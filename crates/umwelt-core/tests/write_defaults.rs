// SPDX-License-Identifier: AGPL-3.0-only
use serde_json::{Value, json};
use umwelt_core::commit_fingerprint;
use umwelt_core::domain::*;

fn claim() -> Value {
    json!({
        "subject": "bob",
        "predicate": "stole",
        "object": { "kind": "entity", "id": "necklace" },
        "polarity": true
    })
}

fn commit(records: Value, transitions: Value) -> CommitInput {
    serde_json::from_value(json!({
        "projectId": "p",
        "attemptId": "a1",
        "event": {
            "id": "e1",
            "type": "scene",
            "title": "The ball",
            "position": { "tick": 7 },
            "provenance": { "origin": { "kind": "human" }, "reason": "draft" }
        },
        "records": records,
        "transitions": transitions
    }))
    .expect("commit input")
}

fn bare_holding() -> Value {
    json!({
        "kind": "holding",
        "id": "h1",
        "holder": "ana",
        "attitude": "knows",
        "claim": claim(),
        "acquisition": { "kind": "witnessed", "eventId": "e1" }
    })
}

fn explicit_holding() -> Value {
    json!({
        "kind": "holding",
        "id": "h1",
        "position": { "tick": 7 },
        "provenance": { "origin": { "kind": "human" }, "reason": "draft" },
        "holder": "ana",
        "attitude": "knows",
        "claim": claim(),
        "acquisition": { "kind": "witnessed", "eventId": "e1" },
        "confidence": 1,
        "accessibility": "available"
    })
}

#[test]
fn omitted_fields_take_the_event_and_the_fixed_defaults() {
    let mut input = commit(
        json!([bare_holding(), { "kind": "fact", "claim": claim() }]),
        json!([{ "targetRecordId": "h1", "status": "forgotten" }]),
    );
    let RecordInput::Holding {
        common,
        confidence,
        accessibility,
        ..
    } = &input.records[0]
    else {
        panic!("expected a holding");
    };
    assert_eq!(*confidence, 1.0);
    assert_eq!(*accessibility, Accessibility::Available);
    assert_eq!(common.position, None);
    assert_eq!(common.position_in(&input.event), Position { tick: 7 });
    assert_eq!(common.provenance_in(&input.event), &input.event.provenance);

    input.normalize();
    for record in &input.records {
        assert_eq!(record.common().position, Some(Position { tick: 7 }));
        assert_eq!(
            record.common().provenance.as_ref(),
            Some(&input.event.provenance)
        );
    }
    assert_eq!(input.transitions[0].position, Some(Position { tick: 7 }));
    assert_eq!(
        input.transitions[0].provenance.as_ref(),
        Some(&input.event.provenance)
    );
}

#[test]
fn explicit_fields_are_never_overwritten() {
    let mut record = bare_holding();
    record["position"] = json!({ "tick": 9 });
    record["provenance"] = json!({ "origin": { "kind": "import" } });
    record["confidence"] = json!(0.25);
    record["accessibility"] = json!("suppressed");
    let mut input = commit(
        json!([record]),
        json!([{ "targetRecordId": "h1", "status": "forgotten", "position": { "tick": 11 } }]),
    );
    input.normalize();
    let RecordInput::Holding {
        common,
        confidence,
        accessibility,
        ..
    } = &input.records[0]
    else {
        panic!("expected a holding");
    };
    assert_eq!(common.position, Some(Position { tick: 9 }));
    assert_eq!(
        common.provenance.as_ref().map(|value| &value.origin),
        Some(&Origin::Import)
    );
    assert_eq!(*confidence, 0.25);
    assert_eq!(*accessibility, Accessibility::Suppressed);
    assert_eq!(input.transitions[0].position, Some(Position { tick: 11 }));
    assert_eq!(
        input.transitions[0].provenance.as_ref(),
        Some(&input.event.provenance)
    );
}

#[test]
fn omitted_and_explicit_requests_share_a_fingerprint_once_normalized() {
    let mut omitted = commit(json!([bare_holding()]), json!([]));
    let mut explicit = commit(json!([explicit_holding()]), json!([]));
    assert_ne!(
        commit_fingerprint(&omitted).expect("fingerprint"),
        commit_fingerprint(&explicit).expect("fingerprint")
    );
    omitted.normalize();
    explicit.normalize();
    assert_eq!(omitted, explicit);
    assert_eq!(
        commit_fingerprint(&omitted).expect("fingerprint"),
        commit_fingerprint(&explicit).expect("fingerprint")
    );

    let mut other = bare_holding();
    other["confidence"] = json!(0.5);
    let mut different = commit(json!([other]), json!([]));
    different.normalize();
    assert_ne!(
        commit_fingerprint(&omitted).expect("fingerprint"),
        commit_fingerprint(&different).expect("fingerprint")
    );
}

#[test]
fn holder_attitude_and_acquisition_stay_required() {
    for field in ["holder", "attitude", "acquisition", "claim"] {
        let mut record = bare_holding();
        record
            .as_object_mut()
            .expect("holding object")
            .remove(field);
        let parsed = serde_json::from_value::<RecordInput>(record);
        assert!(parsed.is_err(), "{field} must stay required");
    }
}

#[test]
fn a_proposal_is_normalized_like_a_commit() {
    let mut input: ProposeInput = serde_json::from_value(json!({
        "projectId": "p",
        "event": {
            "type": "scene",
            "title": "The ball",
            "position": { "tick": 3 },
            "provenance": { "origin": { "kind": "human" } }
        },
        "records": [bare_holding()]
    }))
    .expect("propose input");
    input.normalize();
    assert_eq!(
        input.records[0].common().position,
        Some(Position { tick: 3 })
    );
}
