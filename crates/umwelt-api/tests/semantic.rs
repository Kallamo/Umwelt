// SPDX-License-Identifier: AGPL-3.0-only
//! Semantic search over vectors the client's model made. The engine stores,
//! ranks and filters; it never runs a model.

mod common;

use common::*;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

const SPACE: &str = "test-3d";

fn define_space(engine: &mut TestEngine) {
    engine
        .define_embedding_space(DefineEmbeddingSpaceInput {
            project_id: "p".to_owned(),
            space: EmbeddingSpace {
                id: SPACE.to_owned(),
                dimensions: 3,
                model: "hand-made".to_owned(),
            },
        })
        .expect("define space");
}

fn noted(id: &str, holder: &str, tick: i64, text: &str) -> RecordInput {
    let mut record = holding(id, holder, tick, claim(holder, "related", "item"));
    if let RecordInput::Holding { text: value, .. } = &mut record {
        *value = Some(text.to_owned());
    }
    record
}

/// Alice has three notes and Bob one; there is one Fact and one scene.
fn library() -> TestEngine {
    let mut engine = engine();
    define_space(&mut engine);
    let mut input = commit(
        "day-1",
        1,
        vec![
            noted("necklace", "alice", 1, "someone took the duchess's jewel"),
            noted("weather", "alice", 1, "it rained all night"),
            noted("market", "alice", 1, "bread is expensive at the market"),
            noted("bob-necklace", "bob", 1, "the jewel was stolen"),
            fact("castle", 1, claim("castle", "related", "town")),
        ],
    );
    input.event.title = "The ball at the castle".to_owned();
    engine.commit(input).expect("commit day 1");
    embed_all(&mut engine, None);
    engine
}

/// A hand-made "model": a fixed vector per document, keyed by id.
fn vector_of(doc_id: &str) -> Vec<f32> {
    match doc_id {
        "necklace" => vec![1.0, 0.1, 0.0],
        "bob-necklace" => vec![1.0, 0.0, 0.0],
        "weather" => vec![0.0, 1.0, 0.0],
        "market" => vec![0.2, 0.0, 1.0],
        "castle" => vec![0.0, 0.0, 1.0],
        _ => vec![0.3, 0.3, 0.3],
    }
}

/// Embeds every pending document of `timeline` with the hand-made model.
fn embed_all(engine: &mut TestEngine, timeline: Option<&str>) -> usize {
    let pending = engine
        .pending_embeddings(&PendingEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            timeline_id: timeline.map(str::to_owned),
            doc_types: Vec::new(),
            page: Page::default(),
        })
        .expect("pending");
    let count = pending.items.len();
    let items = pending
        .items
        .into_iter()
        .map(|item| EmbeddingItem {
            vector: vector_of(&item.doc_id),
            doc_type: item.doc_type,
            doc_id: item.doc_id,
            text_hash: item.text_hash,
        })
        .collect::<Vec<_>>();
    engine
        .put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            items,
        })
        .expect("put embeddings");
    count
}

fn semantic(vector: [f32; 3]) -> Option<SearchSemantic> {
    Some(SearchSemantic {
        space_id: SPACE.to_owned(),
        vector: vector.to_vec(),
    })
}

fn holder(id: &str) -> SearchFilter {
    SearchFilter {
        holder: Some(id.to_owned()),
        doc_type: None,
    }
}

fn ids(page: &PageResult<SearchItem>) -> Vec<&str> {
    page.items.iter().map(|item| item.doc_id.as_str()).collect()
}

#[test]
fn pending_lists_the_searchable_text_and_put_clears_it() {
    let mut engine = engine();
    define_space(&mut engine);
    engine
        .commit(commit(
            "day-1",
            1,
            vec![
                noted("necklace", "alice", 1, "someone took the jewel"),
                holding("silent", "alice", 1, claim("alice", "related", "town")),
                fact("castle", 1, claim("castle", "related", "town")),
            ],
        ))
        .expect("commit");
    let pending = engine
        .pending_embeddings(&PendingEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            timeline_id: None,
            doc_types: Vec::new(),
            page: Page::default(),
        })
        .expect("pending");
    let listed = pending
        .items
        .iter()
        .map(|item| {
            (
                item.doc_type.as_str(),
                item.doc_id.as_str(),
                item.text.as_str(),
            )
        })
        .collect::<Vec<_>>();
    // The Holding without text is not searchable; the scene event is.
    assert_eq!(
        listed,
        vec![
            ("event", "e-day-1", "e-day-1"),
            ("record", "necklace", "someone took the jewel"),
            ("record", "castle", "castle related town"),
        ]
    );
    assert_eq!(
        pending.items[1].text_hash,
        umwelt_core::text_hash("someone took the jewel")
    );
    assert!(pending.next_cursor.is_none());

    assert_eq!(embed_all(&mut engine, None), 3);
    let again = engine
        .pending_embeddings(&PendingEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            timeline_id: None,
            doc_types: Vec::new(),
            page: Page::default(),
        })
        .expect("pending");
    assert!(again.items.is_empty());

    // Resuming sees only documents indexed later.
    engine
        .commit(commit(
            "day-2",
            2,
            vec![noted("weather", "alice", 2, "it rained")],
        ))
        .expect("commit day 2");
    let resumed = engine
        .pending_embeddings(&PendingEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            timeline_id: None,
            doc_types: vec!["record".to_owned()],
            page: Page {
                limit: 100,
                cursor: Some(again.resume_cursor),
            },
        })
        .expect("resume");
    let resumed_ids = resumed
        .items
        .iter()
        .map(|item| item.doc_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(resumed_ids, vec!["weather"]);
}

#[test]
fn semantic_search_ranks_by_meaning_within_the_holders_perspective() {
    let engine = library();
    let found = engine
        .search_with(
            "p",
            None,
            semantic([1.0, 0.0, 0.0]),
            holder("alice"),
            at(10),
            Page::default(),
        )
        .expect("semantic search");
    // Bob's note is the closest vector of all, but it is not Alice's.
    assert_eq!(ids(&found), vec!["necklace", "market", "weather"]);
    assert_eq!(found.items[0].matched, Some(SearchMatch::Semantic));
    let score = found.items[0].score.expect("score");
    assert!((score - 0.995_037).abs() < 1e-5, "{score}");
    let record = found.items[0].record.as_ref().map(record_id);
    assert_eq!(record, Some("necklace"));

    // The narrator sees canon and events, never Holdings.
    let narrator = engine
        .search_with(
            "p",
            None,
            semantic([0.0, 0.0, 1.0]),
            SearchFilter::default(),
            at(10),
            Page::default(),
        )
        .expect("narrator search");
    assert_eq!(ids(&narrator), vec!["castle", "e-day-1"]);
    assert!(narrator.items[1].event.is_some());
}

#[test]
fn hybrid_search_fuses_words_and_meaning() {
    let engine = library();
    let found = engine
        .search_with(
            "p",
            Some("market"),
            semantic([1.0, 0.0, 0.0]),
            holder("alice"),
            at(10),
            Page::default(),
        )
        .expect("hybrid search");
    // "market" is first by words and second by meaning: found by both.
    assert_eq!(found.items[0].doc_id, "market");
    assert_eq!(found.items[0].matched, Some(SearchMatch::Both));
    assert_eq!(found.items[1].doc_id, "necklace");
    assert_eq!(found.items[1].matched, Some(SearchMatch::Semantic));

    let text_only = engine
        .search("p", "market", holder("alice"), at(10), Page::default())
        .expect("text search");
    assert_eq!(text_only.items[0].matched, Some(SearchMatch::Text));
    assert!(text_only.items[0].score.is_none());
}

#[test]
fn a_document_the_read_cannot_see_is_never_returned() {
    let mut engine = library();
    // Later knowledge, closer than anything Alice has.
    engine
        .commit(commit(
            "day-5",
            5,
            vec![noted("later", "alice", 5, "the thief was the duke")],
        ))
        .expect("commit day 5");
    embed_all(&mut engine, None);
    engine
        .put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            items: vec![EmbeddingItem {
                doc_type: "record".to_owned(),
                doc_id: "later".to_owned(),
                text_hash: umwelt_core::text_hash("the thief was the duke"),
                vector: vec![1.0, 0.0, 0.0],
            }],
        })
        .expect("replace vector");
    let search = |engine: &TestEngine, tick: i64| {
        engine
            .search_with(
                "p",
                None,
                semantic([1.0, 0.0, 0.0]),
                holder("alice"),
                at(tick),
                Page::default(),
            )
            .expect("search")
    };
    assert!(!ids(&search(&engine, 4)).contains(&"later"));
    assert_eq!(ids(&search(&engine, 5))[0], "later");

    // Forgotten knowledge leaves the results too.
    engine
        .transition_batch(TransitionBatchInput {
            project_id: "p".to_owned(),
            attempt_id: "forget".to_owned(),
            timeline_id: None,
            expected_version: None,
            position: Position { tick: 6 },
            on: AxisPositions::new(),
            status: BatchTransitionStatus::Forgotten,
            record_ids: vec!["later".to_owned()],
            provenance: None,
        })
        .expect("forget");
    assert!(!ids(&search(&engine, 6)).contains(&"later"));
    assert_eq!(ids(&search(&engine, 5))[0], "later");
}

#[test]
fn draft_documents_and_their_vectors_follow_the_draft() {
    let mut engine = library();
    engine
        .open_draft(OpenDraftInput {
            project_id: "p".to_owned(),
            id: Some("d".to_owned()),
            label: None,
            base_timeline_id: None,
        })
        .expect("open draft");
    let mut input = commit(
        "draft-day",
        2,
        vec![noted("draft-note", "alice", 2, "a draft memory")],
    );
    input.timeline_id = Some("d".to_owned());
    engine.commit(input).expect("commit in draft");
    // Main does not even list it as pending; the draft does.
    assert_eq!(embed_all(&mut engine, None), 0);
    assert_eq!(embed_all(&mut engine, Some("d")), 2);
    let in_main = engine
        .search_with(
            "p",
            None,
            semantic([0.3, 0.3, 0.3]),
            holder("alice"),
            at(10),
            Page::default(),
        )
        .expect("search main");
    assert!(!ids(&in_main).contains(&"draft-note"));
    let in_draft = engine
        .in_timeline("d")
        .search_with(
            "p",
            None,
            semantic([0.3, 0.3, 0.3]),
            holder("alice"),
            at(10),
            Page::default(),
        )
        .expect("search draft");
    assert_eq!(ids(&in_draft)[0], "draft-note");

    // A rebase keeps ids, so the vectors still apply.
    engine
        .commit(commit(
            "main-day",
            3,
            vec![fact("f2", 3, claim("town", "related", "castle"))],
        ))
        .expect("main moves on");
    engine.rebase_draft("p", "d").expect("rebase");
    let rebased = engine
        .in_timeline("d")
        .search_with(
            "p",
            None,
            semantic([0.3, 0.3, 0.3]),
            holder("alice"),
            at(10),
            Page::default(),
        )
        .expect("search rebased draft");
    assert_eq!(ids(&rebased)[0], "draft-note");

    // A purge removes the draft's vectors with its rows.
    engine.discard_draft("p", "d", true).expect("purge");
    expect_code(
        engine.put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            items: vec![EmbeddingItem {
                doc_type: "record".to_owned(),
                doc_id: "draft-note".to_owned(),
                text_hash: umwelt_core::text_hash("a draft memory"),
                vector: vec![1.0, 0.0, 0.0],
            }],
        }),
        UmweltErrorCode::NotFound,
    );
    let mut reused = commit(
        "draft-day",
        4,
        vec![noted("draft-note", "alice", 4, "a confirmed memory")],
    );
    reused.event.id = Some("e-draft-day".to_owned());
    engine.commit(reused).expect("ids are free again");
    let pending = engine
        .pending_embeddings(&PendingEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            timeline_id: None,
            doc_types: Vec::new(),
            page: Page::default(),
        })
        .expect("pending");
    let item = pending
        .items
        .iter()
        .find(|item| item.doc_id == "draft-note")
        .expect("the reused id has no vector");
    assert!(!item.stale);
}

#[test]
fn a_vector_made_from_another_text_stays_pending_as_stale() {
    let mut engine = library();
    let result = engine
        .put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            items: vec![EmbeddingItem {
                doc_type: "record".to_owned(),
                doc_id: "weather".to_owned(),
                text_hash: umwelt_core::text_hash("an older text"),
                vector: vec![0.0, 1.0, 0.0],
            }],
        })
        .expect("put stale vector");
    assert_eq!(
        result,
        PutEmbeddingsResult {
            stored: 1,
            stale: 1
        }
    );
    let pending = engine
        .pending_embeddings(&PendingEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            timeline_id: None,
            doc_types: Vec::new(),
            page: Page::default(),
        })
        .expect("pending");
    assert_eq!(pending.items.len(), 1);
    assert_eq!(pending.items[0].doc_id, "weather");
    assert!(pending.items[0].stale);
}

#[test]
fn spaces_and_vectors_are_validated() {
    let mut engine = library();
    let define = |engine: &mut TestEngine, id: &str, dimensions: u32| {
        engine.define_embedding_space(DefineEmbeddingSpaceInput {
            project_id: "p".to_owned(),
            space: EmbeddingSpace {
                id: id.to_owned(),
                dimensions,
                model: "hand-made".to_owned(),
            },
        })
    };
    define(&mut engine, SPACE, 3).expect("same definition is a no-op");
    expect_code(define(&mut engine, SPACE, 4), UmweltErrorCode::Conflict);
    expect_code(
        define(&mut engine, "umwelt:space", 3),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        define(&mut engine, "big", 5000),
        UmweltErrorCode::ValidationFailed,
    );

    let put = |engine: &mut TestEngine, doc_type: &str, doc_id: &str, vector: Vec<f32>| {
        engine.put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            items: vec![EmbeddingItem {
                doc_type: doc_type.to_owned(),
                doc_id: doc_id.to_owned(),
                text_hash: umwelt_core::text_hash("x"),
                vector,
            }],
        })
    };
    expect_code(
        put(&mut engine, "record", "weather", vec![1.0, 0.0]),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        put(&mut engine, "record", "weather", vec![0.0, 0.0, 0.0]),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        put(&mut engine, "entity", "alice", vec![1.0, 0.0, 0.0]),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        put(&mut engine, "record", "missing", vec![1.0, 0.0, 0.0]),
        UmweltErrorCode::NotFound,
    );
    engine
        .commit(commit(
            "silent",
            2,
            vec![holding(
                "silent",
                "alice",
                2,
                claim("alice", "related", "town"),
            )],
        ))
        .expect("commit");
    expect_code(
        put(&mut engine, "record", "silent", vec![1.0, 0.0, 0.0]),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: "missing".to_owned(),
            items: Vec::new(),
        }),
        UmweltErrorCode::NotFound,
    );
    expect_code(
        engine.search_with(
            "p",
            None,
            None,
            SearchFilter::default(),
            At::default(),
            Page::default(),
        ),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.search_with(
            "p",
            None,
            semantic([1.0, 0.0, 0.0]),
            SearchFilter::default(),
            At::default(),
            Page {
                limit: 1000,
                cursor: Some("31".to_owned()),
            },
        ),
        UmweltErrorCode::ValidationFailed,
    );
}

#[test]
fn ties_are_ordered_by_document_and_pages_are_stable() {
    let mut engine = library();
    // Same vector for two of Alice's notes: the tie goes to the smaller id.
    engine
        .put_embeddings(PutEmbeddingsInput {
            project_id: "p".to_owned(),
            space_id: SPACE.to_owned(),
            items: ["market", "weather"]
                .iter()
                .zip(["bread is expensive at the market", "it rained all night"])
                .map(|(id, text)| EmbeddingItem {
                    doc_type: "record".to_owned(),
                    doc_id: (*id).to_owned(),
                    text_hash: umwelt_core::text_hash(text),
                    vector: vec![0.0, 1.0, 1.0],
                })
                .collect(),
        })
        .expect("put");
    let page = |cursor: Option<String>| {
        engine
            .search_with(
                "p",
                None,
                semantic([0.0, 1.0, 1.0]),
                holder("alice"),
                at(10),
                Page { limit: 1, cursor },
            )
            .expect("page")
    };
    let first = page(None);
    assert_eq!(ids(&first), vec!["market"]);
    let second = page(first.next_cursor);
    assert_eq!(ids(&second), vec!["weather"]);
    let third = page(second.next_cursor);
    assert_eq!(ids(&third), vec!["necklace"]);
    assert!(third.next_cursor.is_none());
}

#[test]
fn dropping_a_space_removes_its_vectors_only() {
    let mut engine = library();
    let dropped = engine.drop_embedding_space("p", SPACE).expect("drop space");
    assert_eq!(dropped.removed, 6);
    assert!(engine.embedding_spaces("p").expect("spaces").is_empty());
    expect_code(
        engine.search_with(
            "p",
            None,
            semantic([1.0, 0.0, 0.0]),
            holder("alice"),
            at(10),
            Page::default(),
        ),
        UmweltErrorCode::NotFound,
    );
    // History is untouched.
    let text = engine
        .search("p", "jewel", holder("alice"), at(10), Page::default())
        .expect("text search");
    assert_eq!(ids(&text), vec!["necklace"]);
    let removed = {
        define_space(&mut engine);
        embed_all(&mut engine, None);
        engine
            .delete_embeddings(DeleteEmbeddingsInput {
                project_id: "p".to_owned(),
                space_id: SPACE.to_owned(),
                items: vec![EmbeddingDocRef {
                    doc_type: "record".to_owned(),
                    doc_id: "weather".to_owned(),
                }],
            })
            .expect("delete")
    };
    assert_eq!(removed.removed, 1);
}

#[test]
fn semantic_reads_on_an_additional_axis_follow_the_axis() {
    let mut engine = engine();
    define_space(&mut engine);
    engine
        .define_axis(DefineAxisInput {
            project_id: "p".to_owned(),
            axis: AxisDef {
                id: "reading".to_owned(),
                label: "Reading".to_owned(),
            },
        })
        .expect("define axis");
    let mut chapter = commit(
        "chapter-12",
        3,
        vec![noted("necklace", "alice", 3, "someone took the jewel")],
    );
    chapter.event.on = AxisPositions::from([("reading".to_owned(), 12)]);
    engine.commit(chapter).expect("commit chapter 12");
    embed_all(&mut engine, None);
    let read = |tick: i64| {
        engine
            .search_with(
                "p",
                None,
                semantic([1.0, 0.0, 0.0]),
                holder("alice"),
                At {
                    axis: Some("reading".to_owned()),
                    tick: Some(tick),
                    as_of_version: None,
                },
                Page::default(),
            )
            .expect("search on axis")
    };
    assert!(read(5).items.is_empty());
    assert_eq!(ids(&read(12)), vec!["necklace"]);
}
