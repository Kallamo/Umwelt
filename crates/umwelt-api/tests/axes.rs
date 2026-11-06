// SPDX-License-Identifier: AGPL-3.0-only
//! Additional time axes: events placed in reading order next to story order,
//! and reads on either axis.

mod common;

use common::*;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

const READING: &str = "reading";

fn on(position: i64) -> AxisPositions {
    AxisPositions::from([(READING.to_owned(), position)])
}

fn reading(tick: i64) -> At {
    At {
        axis: Some(READING.to_owned()),
        tick: Some(tick),
        as_of_version: None,
    }
}

fn define_reading(engine: &mut TestEngine) {
    engine
        .define_axis(DefineAxisInput {
            project_id: "p".to_owned(),
            axis: AxisDef {
                id: READING.to_owned(),
                label: "Reading order".to_owned(),
            },
        })
        .expect("define axis");
}

/// A commit whose event sits at story tick `tick` and, when given, at
/// `reading` on the reading axis. Its records are at `tick` too.
fn chapter(
    attempt: &str,
    tick: i64,
    reading: Option<i64>,
    records: Vec<RecordInput>,
) -> CommitInput {
    let mut input = commit(attempt, tick, records);
    if let Some(position) = reading {
        input.event.on = on(position);
    }
    input
}

/// Chapter 1 (story tick 10, read first): alice sees the theft and the reader
/// (carol) reads it. Chapter 12 (story tick 3, read last): bob's flashback,
/// which the reader reads too.
fn novel() -> TestEngine {
    let mut engine = engine();
    define_reading(&mut engine);
    engine
        .commit(chapter(
            "chapter-1",
            10,
            Some(1),
            vec![
                holding("ana-theft", "alice", 10, theft()),
                holding("reader-theft", "carol", 10, theft()),
            ],
        ))
        .expect("chapter 1");
    engine
        .commit(chapter(
            "chapter-12",
            3,
            Some(12),
            vec![
                holding("father-secret", "bob", 3, flashback()),
                holding("reader-secret", "carol", 3, flashback()),
            ],
        ))
        .expect("chapter 12");
    engine
}

fn theft() -> Claim {
    claim("item", "related", "town")
}

fn flashback() -> Claim {
    claim("bob", "related", "castle")
}

fn holds(engine: &TestEngine, holder: &str, value: Claim, at: At) -> bool {
    engine
        .holds("p", holder, ClaimRef::Claim { claim: value }, at)
        .expect("holds")
        .holds
}

#[test]
fn the_reader_and_the_characters_read_different_clocks() {
    let engine = novel();
    // In chapter 5 the reader knows the theft, not the flashback.
    assert!(holds(&engine, "carol", theft(), reading(5)));
    assert!(!holds(&engine, "carol", flashback(), reading(5)));
    // In chapter 12 the reader knows both.
    assert!(holds(&engine, "carol", theft(), reading(12)));
    assert!(holds(&engine, "carol", flashback(), reading(12)));
    // In story time the father knows the secret from tick 3 on.
    assert!(holds(&engine, "bob", flashback(), at(5)));
    assert!(!holds(&engine, "bob", flashback(), at(2)));
    // On the main axis the theft only happens at tick 10.
    assert!(!holds(&engine, "alice", theft(), at(5)));
    assert!(holds(&engine, "alice", theft(), at(10)));
}

#[test]
fn a_record_without_a_position_on_the_axis_is_invisible_there() {
    let mut engine = novel();
    engine
        .commit(commit(
            "aside",
            1,
            vec![holding(
                "reader-aside",
                "carol",
                1,
                claim("carol", "related", "item"),
            )],
        ))
        .expect("commit without reading position");
    let aside = claim("carol", "related", "item");
    assert!(holds(&engine, "carol", aside.clone(), at(1)));
    for position in [0, 1, 12, i64::MAX] {
        assert!(!holds(&engine, "carol", aside.clone(), reading(position)));
    }
    let holdings = engine
        .holdings(
            "p",
            "carol",
            HoldingFilter::default(),
            reading(100),
            Page::default(),
        )
        .expect("holdings");
    let ids = holdings
        .items
        .iter()
        .map(|item| record_id(&item.record))
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["reader-secret", "reader-theft"]);
}

#[test]
fn a_transition_positioned_only_on_the_main_axis_does_not_reach_the_axis() {
    let mut engine = novel();
    engine
        .transition_batch(TransitionBatchInput {
            project_id: "p".to_owned(),
            attempt_id: "forget".to_owned(),
            timeline_id: None,
            expected_version: None,
            position: Position { tick: 11 },
            on: AxisPositions::new(),
            status: TransitionStatus::Forgotten,
            record_ids: vec!["reader-theft".to_owned()],
            provenance: None,
        })
        .expect("forget on the main axis");
    assert!(!holds(&engine, "carol", theft(), at(11)));
    assert!(holds(&engine, "carol", theft(), reading(12)));

    // The same status change placed in chapter 13 applies from there on.
    engine
        .transition_batch(TransitionBatchInput {
            project_id: "p".to_owned(),
            attempt_id: "forget-in-chapter-13".to_owned(),
            timeline_id: None,
            expected_version: None,
            position: Position { tick: 12 },
            on: on(13),
            status: TransitionStatus::Forgotten,
            record_ids: vec!["reader-secret".to_owned()],
            provenance: None,
        })
        .expect("forget in chapter 13");
    assert!(holds(&engine, "carol", flashback(), reading(12)));
    assert!(!holds(&engine, "carol", flashback(), reading(13)));
}

#[test]
fn compare_reads_each_holder_on_its_own_axis() {
    let engine = novel();
    let result = engine
        .compare_across("p", "carol", "alice", reading(12), at(10))
        .expect("compare across axes");
    assert_eq!(result.counts.shared, 1, "{result:?}");
    assert_eq!(result.counts.only_a, 1, "{result:?}");
    assert_eq!(result.counts.only_b, 0, "{result:?}");
    let only_reader = result.only_a[0].a.as_ref().map(record_id);
    assert_eq!(only_reader, Some("reader-secret"));

    // Before chapter 12 the reader knows no more than alice.
    let early = engine
        .compare_across("p", "carol", "alice", reading(5), at(10))
        .expect("compare across axes");
    assert_eq!(early.counts.only_a, 0, "{early:?}");
    assert_eq!(early.counts.shared, 1, "{early:?}");
}

#[test]
fn utterances_and_contexts_speak_of_the_future_on_the_read_axis() {
    let mut engine = novel();
    let checked = engine
        .check_utterance("p", "carol", &[flashback(), theft()], reading(5))
        .expect("check utterance");
    assert_eq!(checked.items[0].verdict, UtteranceVerdict::Future);
    assert_eq!(checked.items[0].record_id.as_deref(), Some("reader-secret"));
    assert_eq!(checked.items[1].verdict, UtteranceVerdict::Holds);

    engine
        .commit(commit(
            "aside",
            1,
            vec![holding(
                "reader-aside",
                "carol",
                1,
                claim("carol", "related", "item"),
            )],
        ))
        .expect("commit without reading position");
    let candidate = |id: &str, record: Option<&str>, event: Option<&str>| ContextCandidate {
        id: id.to_owned(),
        record_id: record.map(str::to_owned),
        event_id: event.map(str::to_owned),
        cost: 1,
        priority: 0,
        classification: None,
    };
    let result = engine
        .context_admit(&ContextAdmitInput {
            project_id: "p".to_owned(),
            timeline_id: None,
            holder: Some("carol".to_owned()),
            at: reading(5),
            candidates: vec![
                candidate("theft", Some("reader-theft"), None),
                candidate("secret", Some("reader-secret"), None),
                candidate("aside", Some("reader-aside"), None),
                candidate("chapter-12", None, Some("e-chapter-12")),
                candidate("aside-event", None, Some("e-aside")),
            ],
            budget: None,
            exclude_classifications: Vec::new(),
            include_withheld: false,
            authorized_narrator: false,
        })
        .expect("context admit");
    let admitted = result
        .admitted
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(admitted, vec!["theft"]);
    let rejected = result
        .rejected
        .iter()
        .map(|item| (item.id.as_str(), item.reason))
        .collect::<Vec<_>>();
    assert_eq!(
        rejected,
        vec![
            ("secret", ContextRejection::Future),
            ("aside", ContextRejection::NotOnAxis),
            ("chapter-12", ContextRejection::Future),
            ("aside-event", ContextRejection::NotOnAxis),
        ]
    );
}

#[test]
fn undefined_axes_are_refused_on_write_and_on_read() {
    let mut engine = novel();
    let mut input = commit("chapter-x", 4, vec![holding("x", "carol", 4, theft())]);
    input.event.on = AxisPositions::from([("chapter".to_owned(), 1)]);
    expect_code(engine.commit(input), UmweltErrorCode::ValidationFailed);
    let mut negative = commit("chapter-y", 4, vec![holding("y", "carol", 4, theft())]);
    negative.event.on = on(-1);
    expect_code(engine.commit(negative), UmweltErrorCode::ValidationFailed);
    expect_code(
        engine.holds(
            "p",
            "carol",
            ClaimRef::Claim { claim: theft() },
            At {
                axis: Some("chapter".to_owned()),
                tick: Some(1),
                as_of_version: None,
            },
        ),
        UmweltErrorCode::ValidationFailed,
    );
}

#[test]
fn axes_are_declared_once_and_never_renamed() {
    let mut engine = engine();
    for id in ["tick", "umwelt:reading", " "] {
        expect_code(
            engine.define_axis(DefineAxisInput {
                project_id: "p".to_owned(),
                axis: AxisDef {
                    id: id.to_owned(),
                    label: "Reserved".to_owned(),
                },
            }),
            UmweltErrorCode::ValidationFailed,
        );
    }
    define_reading(&mut engine);
    define_reading(&mut engine);
    expect_code(
        engine.define_axis(DefineAxisInput {
            project_id: "p".to_owned(),
            axis: AxisDef {
                id: READING.to_owned(),
                label: "Another label".to_owned(),
            },
        }),
        UmweltErrorCode::Conflict,
    );
    assert_eq!(
        engine.axes("p").expect("axes"),
        vec![AxisDef {
            id: READING.to_owned(),
            label: "Reading order".to_owned(),
        }]
    );
    expect_code(
        engine.define_axis(DefineAxisInput {
            project_id: "missing".to_owned(),
            axis: AxisDef {
                id: READING.to_owned(),
                label: "Reading order".to_owned(),
            },
        }),
        UmweltErrorCode::NotFound,
    );
}

#[test]
fn the_default_tick_on_an_axis_is_its_highest_position() {
    let engine = novel();
    let latest = At {
        axis: Some(READING.to_owned()),
        tick: None,
        as_of_version: None,
    };
    assert!(holds(&engine, "carol", flashback(), latest));
}

#[test]
fn the_event_page_on_an_axis_is_in_reading_order() {
    let engine = novel();
    let story = engine
        .timeline("p", at(100), Page::default())
        .expect("story order");
    let story_ids = story
        .items
        .iter()
        .map(|event| event.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(story_ids, vec!["e-chapter-12", "e-chapter-1"]);

    let first = engine
        .timeline(
            "p",
            reading(100),
            Page {
                limit: 1,
                cursor: None,
            },
        )
        .expect("first page");
    assert_eq!(first.items[0].id, "e-chapter-1");
    assert_eq!(first.items[0].on, on(1));
    let second = engine
        .timeline(
            "p",
            reading(100),
            Page {
                limit: 1,
                cursor: first.next_cursor,
            },
        )
        .expect("second page");
    assert_eq!(second.items[0].id, "e-chapter-12");
    assert!(second.next_cursor.is_none());

    let early = engine
        .timeline("p", reading(5), Page::default())
        .expect("early page");
    assert_eq!(early.items.len(), 1);
}

#[test]
fn engine_built_events_carry_the_axis_positions() {
    let mut engine = novel();
    let told = engine
        .transmit(TransmitInput {
            project_id: "p".to_owned(),
            attempt_id: "tell".to_owned(),
            timeline_id: None,
            position: Position { tick: 11 },
            on: on(2),
            from: "alice".to_owned(),
            to: "bob".to_owned(),
            source_record_id: "ana-theft".to_owned(),
            record_id: Some("bob-theft".to_owned()),
            attitude: Attitude::Believes,
            confidence: 0.8,
            intent: Intent::Honest,
            distorted_claim: None,
            text: None,
        })
        .expect("transmit in chapter 2");
    assert_eq!(engine.event("p", &told.event_id).expect("event").on, on(2));
    assert!(holds(&engine, "bob", theft(), reading(2)));
    assert!(!holds(&engine, "bob", theft(), reading(1)));

    engine
        .adjust_holdings(AdjustHoldingsInput {
            project_id: "p".to_owned(),
            attempt_id: "doubt".to_owned(),
            timeline_id: None,
            expected_version: None,
            position: Position { tick: 12 },
            on: on(3),
            adjustments: vec![HoldingAdjustment {
                record_id: "bob-theft".to_owned(),
                new_record_id: Some("bob-theft-doubt".to_owned()),
                confidence: Some(0.2),
                accessibility: None,
                attitude: None,
            }],
            provenance: None,
        })
        .expect("adjust in chapter 3");
    let in_chapter_2 = engine
        .holds("p", "bob", ClaimRef::Claim { claim: theft() }, reading(2))
        .expect("holds");
    assert_eq!(in_chapter_2.record_id.as_deref(), Some("bob-theft"));
    let in_chapter_3 = engine
        .holds("p", "bob", ClaimRef::Claim { claim: theft() }, reading(3))
        .expect("holds");
    assert_eq!(in_chapter_3.record_id.as_deref(), Some("bob-theft-doubt"));
}

#[test]
fn drafts_keep_their_positions_through_promote_rebase_and_purge() {
    let mut engine = novel();
    let open = |engine: &mut TestEngine, id: &str| {
        engine
            .open_draft(OpenDraftInput {
                project_id: "p".to_owned(),
                id: Some(id.to_owned()),
                label: None,
                base_timeline_id: None,
            })
            .expect("open draft");
    };
    let in_draft = |attempt: &str, draft: &str, reading: i64, id: &str| {
        let mut input = chapter(
            attempt,
            20,
            Some(reading),
            vec![holding(
                id,
                "carol",
                20,
                claim("carol", "related", "castle"),
            )],
        );
        input.timeline_id = Some(draft.to_owned());
        input
    };
    let draft_claim = || claim("carol", "related", "castle");

    // Promote: the draft's chapter 13 reaches main with its position.
    open(&mut engine, "next-chapter");
    engine
        .commit(in_draft("chapter-13", "next-chapter", 13, "reader-castle"))
        .expect("commit in draft");
    let latest = At {
        axis: Some(READING.to_owned()),
        tick: None,
        as_of_version: None,
    };
    assert!(!holds(&engine, "carol", draft_claim(), latest.clone()));
    assert!(
        engine
            .in_timeline("next-chapter")
            .holds(
                "p",
                "carol",
                ClaimRef::Claim {
                    claim: draft_claim()
                },
                latest.clone()
            )
            .expect("draft holds")
            .holds
    );
    engine
        .promote_draft("p", "next-chapter")
        .expect("promote draft");
    assert!(holds(&engine, "carol", draft_claim(), reading(13)));
    assert!(!holds(&engine, "carol", draft_claim(), reading(12)));

    // Rebase: the draft's events are written again with their positions.
    open(&mut engine, "rewrite");
    engine
        .commit(in_draft("chapter-14", "rewrite", 14, "reader-castle-2"))
        .expect("commit in draft");
    engine
        .commit(chapter(
            "chapter-2",
            11,
            Some(2),
            vec![holding(
                "ana-castle",
                "alice",
                11,
                claim("alice", "related", "castle"),
            )],
        ))
        .expect("main moves on");
    engine.rebase_draft("p", "rewrite").expect("rebase draft");
    let page = engine
        .in_timeline("rewrite")
        .timeline("p", reading(100), Page::default())
        .expect("draft event page");
    let ids = page
        .items
        .iter()
        .map(|event| event.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "e-chapter-1",
            "e-chapter-2",
            "e-chapter-12",
            "e-chapter-13",
            "e-chapter-14"
        ]
    );

    // Purge: the draft's positions go with its events.
    engine
        .discard_draft("p", "rewrite", true)
        .expect("discard and purge");
    let main_page = engine
        .timeline("p", reading(100), Page::default())
        .expect("main event page");
    assert_eq!(main_page.items.len(), 4);
    // The purged event id is free again; a leftover position would collide
    // with the new one on the `event_positions` key.
    engine
        .commit(chapter(
            "chapter-14",
            21,
            Some(14),
            vec![holding("reader-again", "carol", 21, theft())],
        ))
        .expect("reuse the purged event id");
    let reused = engine.event("p", "e-chapter-14").expect("event");
    assert_eq!(reused.on, on(14));
    assert_eq!(reused.position.tick, 21);
}
