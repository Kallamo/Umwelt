// SPDX-License-Identifier: AGPL-3.0-only
//! check.utterance: claims checked against what a holder can know, and canon.

mod common;

use common::*;
use umwelt_core::domain::*;

fn check(engine: &TestEngine, holder: &str, tick: i64, claims: Vec<Claim>) -> CheckUtteranceResult {
    engine
        .check_utterance("p", holder, &claims, at(tick))
        .expect("check")
}

fn story() -> TestEngine {
    let mut engine = engine();
    let mut suppressed = holding("h-buried", "alice", 1, claim("bob", "related", "item"));
    if let RecordInput::Holding { accessibility, .. } = &mut suppressed {
        *accessibility = Accessibility::Suppressed;
    }
    engine
        .commit(commit(
            "day1",
            1,
            vec![
                fact("f-location", 1, claim("carol", "location", "castle")),
                holding("h-loc", "alice", 1, claim("carol", "location", "town")),
                holding("h-rel", "alice", 1, claim("castle", "related", "item")),
                holding(
                    "h-not",
                    "alice",
                    1,
                    negated(claim("carol", "related", "item")),
                ),
                suppressed,
            ],
        ))
        .expect("day 1");
    engine
        .commit(commit(
            "day5",
            5,
            vec![holding(
                "h-later",
                "alice",
                5,
                claim("bob", "related", "castle"),
            )],
        ))
        .expect("day 5");
    engine
}

#[test]
fn each_claim_gets_a_verdict_and_the_record_behind_it() {
    let engine = story();
    let result = check(
        &engine,
        "alice",
        2,
        vec![
            claim("castle", "related", "item"),
            claim("carol", "related", "item"),
            claim("carol", "location", "castle"),
            claim("bob", "related", "item"),
            claim("bob", "related", "castle"),
            claim("bob", "related", "town"),
        ],
    );
    let verdicts = result
        .items
        .iter()
        .map(|item| (item.verdict, item.record_id.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        verdicts,
        [
            (UtteranceVerdict::Holds, Some("h-rel")),
            (UtteranceVerdict::HoldsOpposite, Some("h-not")),
            (UtteranceVerdict::HoldsOtherValue, Some("h-loc")),
            (UtteranceVerdict::Withheld, Some("h-buried")),
            (UtteranceVerdict::Future, Some("h-later")),
            (UtteranceVerdict::Unknown, None),
        ]
    );
    assert_eq!(result.items[0].attitude, Some(Attitude::Believes));
    assert!(!result.consistent);
}

#[test]
fn canon_is_reported_separately_from_the_holder() {
    let engine = story();
    let result = check(
        &engine,
        "alice",
        2,
        vec![
            claim("carol", "location", "castle"),
            claim("carol", "location", "town"),
            negated(claim("carol", "location", "castle")),
            claim("castle", "related", "item"),
        ],
    );
    let canon = result
        .items
        .iter()
        .map(|item| (item.canon, item.canon_record_id.as_deref()))
        .collect::<Vec<_>>();
    assert_eq!(
        canon,
        [
            (CanonStatus::Agrees, Some("f-location")),
            (CanonStatus::Contradicts, Some("f-location")),
            (CanonStatus::Contradicts, Some("f-location")),
            (CanonStatus::NoCanon, None),
        ]
    );
    assert_eq!(
        result.items[0].verdict,
        UtteranceVerdict::HoldsOtherValue,
        "canon being true does not mean alice knows it"
    );
}

#[test]
fn a_consistent_line_holds_everything_it_says() {
    let engine = story();
    let early = check(
        &engine,
        "alice",
        2,
        vec![claim("castle", "related", "item")],
    );
    assert!(early.consistent);
    let anachronism = check(&engine, "alice", 2, vec![claim("bob", "related", "castle")]);
    assert!(!anachronism.consistent);
    let later = check(&engine, "alice", 5, vec![claim("bob", "related", "castle")]);
    assert!(later.consistent, "by day 5 alice knows it");
}
