// SPDX-License-Identifier: AGPL-3.0-only
//! Batch maintenance: adjusted Holdings, batch transitions, dispositions.

mod common;

use common::*;
use serde_json::json;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn rumor() -> Claim {
    claim("castle", "related", "item")
}

fn seeded() -> TestEngine {
    let mut engine = engine();
    engine
        .commit(commit(
            "day1",
            1,
            vec![
                holding("h-alice", "alice", 1, rumor()),
                holding("h-bob", "bob", 1, rumor()),
            ],
        ))
        .expect("seed");
    engine
}

fn adjust(attempt: &str, tick: i64, adjustments: Vec<HoldingAdjustment>) -> AdjustHoldingsInput {
    AdjustHoldingsInput {
        on: Default::default(),
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: None,
        expected_version: None,
        position: Position { tick },
        adjustments,
        provenance: None,
    }
}

fn fade(record_id: &str, confidence: f64) -> HoldingAdjustment {
    HoldingAdjustment {
        record_id: record_id.to_owned(),
        new_record_id: None,
        confidence: Some(confidence),
        accessibility: None,
        attitude: None,
    }
}

fn holds(engine: &TestEngine, holder: &str, tick: i64) -> HoldsResult {
    engine
        .holds("p", holder, ClaimRef::Claim { claim: rumor() }, at(tick))
        .expect("holds")
}

#[test]
fn adjust_batch_supersedes_with_copies_and_keeps_history() {
    let mut engine = seeded();
    let mut buried = fade("h-bob", 0.9);
    buried.accessibility = Some(Accessibility::Suppressed);
    buried.new_record_id = Some("h-bob-2".to_owned());
    let outcome = engine
        .adjust_holdings(adjust("night1", 2, vec![fade("h-alice", 0.5), buried]))
        .expect("adjust");
    assert_eq!(outcome.record_ids.len(), 2);
    assert_eq!(outcome.transition_ids.len(), 2);

    let alice = holds(&engine, "alice", 2);
    assert_eq!(alice.confidence, Some(0.5));
    assert_ne!(alice.record_id.as_deref(), Some("h-alice"));
    assert!(
        !holds(&engine, "bob", 2).holds,
        "suppressed is not available"
    );
    assert_eq!(
        holds(&engine, "alice", 1).confidence,
        Some(1.0),
        "history is kept"
    );

    let replaced = engine
        .record("p", alice.record_id.as_deref().expect("id"))
        .expect("replacement");
    assert!(matches!(
        replaced,
        Record::Holding { acquisition: Acquisition::Authored, claim, .. } if claim == rumor()
    ));
    let trace = engine.trace("p", "h-alice").expect("trace");
    assert_eq!(trace.status, Some(TransitionStatus::Superseded));
}

#[test]
fn adjust_batch_is_idempotent_and_atomic() {
    let mut engine = seeded();
    let first = engine
        .adjust_holdings(adjust("night1", 2, vec![fade("h-alice", 0.5)]))
        .expect("adjust");
    let replay = engine
        .adjust_holdings(adjust("night1", 2, vec![fade("h-alice", 0.5)]))
        .expect("replay");
    assert_eq!(first, replay);

    let before = engine.counts("p").expect("counts");
    expect_code(
        engine.adjust_holdings(adjust(
            "night2",
            3,
            vec![fade("h-bob", 0.2), fade("h-alice", 0.1)],
        )),
        UmweltErrorCode::ValidationFailed,
    );
    assert_eq!(
        engine.counts("p").expect("counts"),
        before,
        "h-alice is already superseded"
    );
    assert_eq!(holds(&engine, "bob", 3).confidence, Some(1.0));
}

#[test]
fn invalid_adjustments_are_rejected() {
    let mut engine = seeded();
    let mut nothing = fade("h-alice", 0.5);
    nothing.confidence = None;
    expect_code(
        engine.adjust_holdings(adjust("a", 2, vec![nothing])),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.adjust_holdings(adjust(
            "b",
            2,
            vec![fade("h-alice", 0.5), fade("h-alice", 0.4)],
        )),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.adjust_holdings(adjust("c", 2, vec![fade("missing", 0.5)])),
        UmweltErrorCode::NotFound,
    );
    expect_code(
        engine.adjust_holdings(adjust("d", 2, vec![fade("h-alice", 1.5)])),
        UmweltErrorCode::ValidationFailed,
    );
}

#[test]
fn transition_batch_forgets_many_records_at_once() {
    let mut engine = seeded();
    engine
        .transition_batch(TransitionBatchInput {
            on: Default::default(),
            project_id: "p".to_owned(),
            attempt_id: "forget".to_owned(),
            timeline_id: None,
            expected_version: None,
            position: Position { tick: 3 },
            status: TransitionStatus::Forgotten,
            record_ids: vec!["h-alice".to_owned(), "h-bob".to_owned()],
            provenance: None,
        })
        .expect("forget");
    assert!(!holds(&engine, "alice", 3).holds);
    assert!(!holds(&engine, "bob", 3).holds);
    assert!(holds(&engine, "bob", 2).holds);
    expect_code(
        engine.transition_batch(TransitionBatchInput {
            on: Default::default(),
            project_id: "p".to_owned(),
            attempt_id: "supersede".to_owned(),
            timeline_id: None,
            expected_version: None,
            position: Position { tick: 3 },
            status: TransitionStatus::Superseded,
            record_ids: vec!["h-alice".to_owned()],
            provenance: None,
        }),
        UmweltErrorCode::ValidationFailed,
    );
}

#[test]
fn disposition_batch_keeps_the_latest_value_per_dimension() {
    let mut engine = seeded();
    let set = |attempt: &str, tick: i64, value: f64| SetDispositionsInput {
        on: Default::default(),
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: None,
        expected_version: None,
        position: Position { tick },
        dispositions: vec![
            DispositionSetting {
                id: None,
                holder: "alice".to_owned(),
                target: "bob".to_owned(),
                dimension: "trust".to_owned(),
                value: json!(value),
            },
            DispositionSetting {
                id: None,
                holder: "bob".to_owned(),
                target: "alice".to_owned(),
                dimension: "trust".to_owned(),
                value: json!(0.1),
            },
        ],
        provenance: None,
    };
    engine.set_dispositions(set("t1", 2, 0.8)).expect("first");
    engine.set_dispositions(set("t2", 3, 0.3)).expect("second");
    let alice = engine
        .perspective("p", "alice", At::default(), Page::default())
        .expect("perspective");
    assert_eq!(alice.dispositions.len(), 1);
    assert!(
        matches!(&alice.dispositions[0], Record::Disposition { value, .. } if *value == json!(0.3))
    );
    let earlier = engine
        .perspective("p", "alice", at(2), Page::default())
        .expect("earlier perspective");
    assert!(
        matches!(&earlier.dispositions[0], Record::Disposition { value, .. } if *value == json!(0.8))
    );
}
