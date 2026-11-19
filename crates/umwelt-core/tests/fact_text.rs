// SPDX-License-Identifier: AGPL-3.0-only
use serde_json::json;
use umwelt_core::domain::*;
use umwelt_core::{commit_fingerprint, record_search_text};

fn commit(fact: serde_json::Value) -> CommitInput {
    serde_json::from_value(json!({
        "projectId": "p",
        "attemptId": "a1",
        "event": { "type": "scene", "title": "Fire", "position": { "tick": 1 }, "provenance": { "origin": { "kind": "human" } } },
        "records": [fact]
    }))
    .expect("commit")
}

fn fact(text: Option<&str>) -> serde_json::Value {
    let mut value = json!({
        "kind": "fact",
        "claim": {
            "subject": "mill",
            "predicate": "burned-on",
            "object": { "kind": "value", "value": { "day": 3 } },
            "polarity": true
        }
    });
    if let Some(text) = text {
        value["text"] = json!(text);
    }
    value
}

/// A Fact without `text` serializes exactly as before the field existed
/// (0.1.0), so its commit keeps its fingerprint and retries of attempts stored
/// by 0.1.0 still replay. If this value changes, old attempts would conflict.
const FINGERPRINT_WITHOUT_TEXT: &str =
    "d1789392a99ffd56d61a73b02515e1deea6c26b548a03d0cedf24f0e8f441956";

#[test]
fn a_fact_without_text_keeps_the_fingerprint_of_0_1_0() {
    let fingerprint = commit_fingerprint(&commit(fact(None))).expect("fingerprint");
    assert_eq!(fingerprint, FINGERPRINT_WITHOUT_TEXT);
    let worded = commit_fingerprint(&commit(fact(Some("The mill burned")))).expect("fingerprint");
    assert_ne!(worded, fingerprint);
}

#[test]
fn the_text_of_a_fact_replaces_its_claim_in_search() {
    let record = |text: Option<&str>| -> Record {
        let mut value = fact(text);
        let fields = value.as_object_mut().expect("object");
        fields.insert("id".into(), json!("f1"));
        fields.insert("projectId".into(), json!("p"));
        fields.insert("timelineId".into(), json!("main"));
        fields.insert("position".into(), json!({ "tick": 1 }));
        fields.insert("recordedAt".into(), json!(1));
        fields.insert("eventId".into(), json!("e1"));
        fields.insert(
            "provenance".into(),
            json!({ "origin": { "kind": "human" } }),
        );
        serde_json::from_value(value).expect("record")
    };
    assert_eq!(
        record_search_text(&record(None)).as_deref(),
        Some(r#"mill burned on {"day":3}"#)
    );
    assert_eq!(
        record_search_text(&record(Some("The mill burned on the third day"))).as_deref(),
        Some("The mill burned on the third day")
    );
}
