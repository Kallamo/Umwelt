// SPDX-License-Identifier: AGPL-3.0-only
#![deny(clippy::unwrap_used, clippy::expect_used)]
use std::{hint::black_box, path::PathBuf, sync::Arc, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use umwelt_api::*;

#[allow(dead_code)]
#[path = "../../../apps/cli/src/bench_gen.rs"]
mod generator;

trait BenchmarkResult<T> {
    fn required(self, operation: &str) -> T;
}

impl<T, E: std::fmt::Display> BenchmarkResult<T> for Result<T, E> {
    fn required(self, operation: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("Benchmark {operation} failed: {error}"),
        }
    }
}

/// `holds` and `holdings` on the `reading` axis, on a fixture whose
/// events all have a reading position (`bench-gen --reading-axis`). Only run
/// with `UMWELT_BENCH_AXES=1`: the fixture takes as long to build as the main
/// one. There is no target; the numbers document the cost of axis reads.
fn axis_benchmarks(criterion: &mut Criterion, root: &std::path::Path) {
    if std::env::var_os("UMWELT_BENCH_AXES").is_none() {
        return;
    }
    let dataset = root.join("target/town-reading.umw");
    if !dataset.exists() {
        generator::generate_with(
            &dataset,
            "town",
            42,
            generator::Extras {
                reading_axis: true,
                embeddings: None,
            },
        )
        .required("generate town-reading");
    }
    let engine = Engine::new(
        SqliteStore::open(&dataset).required("open reading fixture"),
        Arc::new(FixedClock("2026-01-01T00:00:00Z".into())),
        Arc::new(UuidV7Gen),
    );
    let claim_ref = ClaimRef::Claim {
        claim: generator::benchmark_claim(0, 1),
    };
    let reading = At {
        axis: Some(generator::READING_AXIS.into()),
        tick: None,
        as_of_version: None,
    };
    let mut group = criterion.benchmark_group("town-reading");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    for (name, at) in [("main_axis", At::default()), ("reading_axis", reading)] {
        group.bench_function(format!("holds_{name}"), |b| {
            b.iter(|| {
                black_box(
                    engine
                        .holds("town", "agent-00", claim_ref.clone(), at.clone())
                        .required("holds"),
                )
            })
        });
        group.bench_function(format!("holdings_about_50_{name}"), |b| {
            b.iter(|| {
                black_box(
                    engine
                        .holdings(
                            "town",
                            "agent-00",
                            HoldingFilter {
                                about: Some("agent-01".into()),
                                ..Default::default()
                            },
                            at.clone(),
                            Page {
                                limit: 50,
                                cursor: None,
                            },
                        )
                        .required("holdings"),
                )
            })
        });
    }
    group.finish();
}

/// Semantic and hybrid search over 384-dimension vectors of every
/// searchable document (`bench-gen --embeddings 384`), and storing 1,000
/// vectors. Only run with `UMWELT_BENCH_EMBEDDINGS=1`: the fixture takes
/// longer to build than the main one.
fn embedding_benchmarks(criterion: &mut Criterion, root: &std::path::Path) {
    if std::env::var_os("UMWELT_BENCH_EMBEDDINGS").is_none() {
        return;
    }
    const DIMENSIONS: u32 = 384;
    let dataset = root.join("target/town-embeddings.umw");
    if !dataset.exists() {
        generator::generate_with(
            &dataset,
            "town",
            42,
            generator::Extras {
                reading_axis: false,
                embeddings: Some(DIMENSIONS),
            },
        )
        .required("generate town-embeddings");
    }
    let working_directory =
        tempfile::tempdir_in(root.join("target")).required("benchmark directory");
    let working = working_directory.path().join("embeddings.umw");
    std::fs::copy(&dataset, &working).required("copy embeddings fixture");
    let mut engine = Engine::new(
        SqliteStore::open(&working).required("open embeddings fixture"),
        Arc::new(FixedClock("2026-01-01T00:00:00Z".into())),
        Arc::new(UuidV7Gen),
    );
    let mut random = generator::SplitMix64::new(7);
    let mut vector = || {
        (0..DIMENSIONS)
            .map(|_| (random.next() as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32)
            .collect::<Vec<_>>()
    };
    let query = vector();
    let semantic = || {
        Some(SearchSemantic {
            space_id: generator::EMBEDDING_SPACE.into(),
            vector: query.clone(),
        })
    };
    let page = || Page {
        limit: 20,
        cursor: None,
    };
    let mut group = criterion.benchmark_group("town-embeddings");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    group.bench_function("semantic_holder", |b| {
        b.iter(|| {
            black_box(
                engine
                    .search_with(
                        "town",
                        None,
                        semantic(),
                        SearchFilter {
                            holder: Some("agent-00".into()),
                            doc_type: None,
                        },
                        At::default(),
                        page(),
                    )
                    .required("semantic search"),
            )
        })
    });
    group.bench_function("semantic_narrator", |b| {
        b.iter(|| {
            black_box(
                engine
                    .search_with(
                        "town",
                        None,
                        semantic(),
                        SearchFilter::default(),
                        At::default(),
                        page(),
                    )
                    .required("semantic search"),
            )
        })
    });
    group.bench_function("hybrid_holder", |b| {
        b.iter(|| {
            black_box(
                engine
                    .search_with(
                        "town",
                        Some("gift"),
                        semantic(),
                        SearchFilter {
                            holder: Some("agent-00".into()),
                            doc_type: None,
                        },
                        At::default(),
                        page(),
                    )
                    .required("hybrid search"),
            )
        })
    });
    let ids = (1..=10)
        .flat_map(|index| (0..100).map(move |item| format!("authored-{index:04}-{item:03}")))
        .collect::<Vec<_>>();
    let text_hash = umwelt_core::text_hash("A gift was exchanged in town.");
    let batch = ids
        .iter()
        .map(|id| EmbeddingItem {
            doc_type: "record".into(),
            doc_id: id.clone(),
            text_hash: text_hash.clone(),
            vector: vector(),
        })
        .collect::<Vec<_>>();
    group.bench_function("put_1000", |b| {
        b.iter(|| {
            black_box(
                engine
                    .put_embeddings(PutEmbeddingsInput {
                        project_id: "town".into(),
                        space_id: generator::EMBEDDING_SPACE.into(),
                        items: batch.clone(),
                    })
                    .required("put embeddings"),
            )
        })
    });
    group.finish();
}

/// Reading one Disposition directly against reading the holder's whole
/// perspective to find it. Only run with `UMWELT_BENCH_DISPOSITIONS=1`. Every
/// agent gets a trust and a fear toward every other agent, set on three days,
/// on a copy of the main fixture. There is no target; the numbers document
/// what `dispositions` saves.
fn disposition_benchmarks(criterion: &mut Criterion, root: &std::path::Path) {
    if std::env::var_os("UMWELT_BENCH_DISPOSITIONS").is_none() {
        return;
    }
    let dataset = root.join("target/town.umw");
    if !dataset.exists() {
        generator::generate_with(&dataset, "town", 42, generator::Extras::default())
            .required("generate town");
    }
    let working_directory =
        tempfile::tempdir_in(root.join("target")).required("benchmark directory");
    let working = working_directory.path().join("dispositions.umw");
    std::fs::copy(&dataset, &working).required("copy checkpointed fixture");
    let mut engine = Engine::new(
        SqliteStore::open(&working).required("open fixture"),
        Arc::new(FixedClock("2026-01-01T00:00:00Z".into())),
        Arc::new(UuidV7Gen),
    );
    let agent = |index: usize| format!("agent-{index:02}");
    for (round, tick) in [10_i64, 1_000, 3_000].into_iter().enumerate() {
        let dispositions = (0..40)
            .flat_map(|holder| (0..40).map(move |target| (holder, target)))
            .filter(|(holder, target)| holder != target)
            .flat_map(|(holder, target)| {
                ["trust", "fear"].map(|dimension| DispositionSetting {
                    id: None,
                    holder: agent(holder),
                    target: agent(target),
                    dimension: dimension.to_owned(),
                    value: serde_json::json!(((holder + target + round) % 10) as f64 / 10.0),
                })
            })
            .collect();
        engine
            .set_dispositions(SetDispositionsInput {
                project_id: "town".into(),
                attempt_id: format!("bench-dispositions-{round}"),
                timeline_id: None,
                expected_version: None,
                position: Position { tick },
                on: Default::default(),
                dispositions,
                provenance: None,
            })
            .required("set dispositions");
    }
    let mut group = criterion.benchmark_group("town-dispositions");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    group.bench_function("dispositions_one", |b| {
        b.iter(|| {
            black_box(
                engine
                    .dispositions(
                        "town",
                        "agent-00",
                        Some("agent-07"),
                        Some("trust"),
                        At::default(),
                    )
                    .required("dispositions"),
            )
        })
    });
    group.bench_function("dispositions_all", |b| {
        b.iter(|| {
            black_box(
                engine
                    .dispositions("town", "agent-00", None, None, At::default())
                    .required("dispositions"),
            )
        })
    });
    group.bench_function("perspective", |b| {
        b.iter(|| {
            black_box(
                engine
                    .perspective("town", "agent-00", At::default(), Page::default())
                    .required("perspective"),
            )
        })
    });
    group.finish();
}

fn benchmarks(criterion: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    axis_benchmarks(criterion, &root);
    embedding_benchmarks(criterion, &root);
    disposition_benchmarks(criterion, &root);
    let dataset = root.join("target/town.umw");
    if !dataset.exists() {
        generator::generate_with(&dataset, "town", 42, generator::Extras::default())
            .required("generate town");
    }
    let working_directory =
        tempfile::tempdir_in(root.join("target")).required("benchmark directory");
    let working = working_directory.path().join("working.umw");
    std::fs::copy(&dataset, &working).required("copy checkpointed fixture");
    let mut engine = Engine::new(
        SqliteStore::open(&working).required("open fixture"),
        Arc::new(FixedClock("2026-01-01T00:00:00Z".into())),
        Arc::new(UuidV7Gen),
    );
    let claim = generator::benchmark_claim(0, 1);
    let claim_ref = ClaimRef::Claim {
        claim: claim.clone(),
    };
    let mut group = criterion.benchmark_group("town");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    group.bench_function("holds", |b| {
        b.iter(|| {
            black_box(
                engine
                    .holds("town", "agent-00", claim_ref.clone(), At::default())
                    .required("holds"),
            )
        })
    });
    group.bench_function("holdings_about_50", |b| {
        b.iter(|| {
            black_box(
                engine
                    .holdings(
                        "town",
                        "agent-00",
                        HoldingFilter {
                            about: Some("agent-01".into()),
                            ..Default::default()
                        },
                        At::default(),
                        Page {
                            limit: 50,
                            cursor: None,
                        },
                    )
                    .required("holdings"),
            )
        })
    });
    group.bench_function("who_holds", |b| {
        b.iter(|| {
            black_box(
                engine
                    .who_holds(
                        "town",
                        claim_ref.clone(),
                        At::default(),
                        WhoHoldsFilter::default(),
                    )
                    .required("who holds"),
            )
        })
    });
    // The same reads through empty drafts one and three deep: every row comes
    // from main through the draft's ancestry. Opened before the
    // writes below, so they see the fixture as generated.
    let mut parent = "main".to_owned();
    for depth in 1..=3 {
        let id = format!("bench-draft-{depth}");
        engine
            .open_draft(OpenDraftInput {
                project_id: "town".into(),
                id: Some(id.clone()),
                label: None,
                base_timeline_id: Some(parent),
            })
            .required("open draft");
        parent = id;
    }
    for depth in [1, 3] {
        let timeline = format!("bench-draft-{depth}");
        group.bench_function(format!("holds_draft_depth_{depth}"), |b| {
            b.iter(|| {
                black_box(
                    engine
                        .in_timeline(&timeline)
                        .holds("town", "agent-00", claim_ref.clone(), At::default())
                        .required("holds in draft"),
                )
            })
        });
        group.bench_function(format!("holdings_about_50_draft_depth_{depth}"), |b| {
            b.iter(|| {
                black_box(
                    engine
                        .in_timeline(&timeline)
                        .holdings(
                            "town",
                            "agent-00",
                            HoldingFilter {
                                about: Some("agent-01".into()),
                                ..Default::default()
                            },
                            At::default(),
                            Page {
                                limit: 50,
                                cursor: None,
                            },
                        )
                        .required("holdings in draft"),
                )
            })
        });
    }
    let mut sequence = 0_u64;
    group.bench_function("commit", |b| {
        b.iter(|| {
            sequence += 1;
            let id = format!("bench-single-{sequence}");
            black_box(
                engine
                    .commit(generator::workload_commit(
                        id.clone(),
                        3361,
                        vec![generator::holding(
                            id,
                            "agent-00".into(),
                            3361,
                            claim.clone(),
                        )],
                    ))
                    .required("commit"),
            )
        })
    });
    group.bench_function("commit_batch_100", |b| {
        b.iter(|| {
            sequence += 1;
            let commits = (0..100)
                .map(|index| {
                    let id = format!("bench-batch-{sequence}-{index}");
                    generator::workload_commit(
                        id.clone(),
                        3361,
                        vec![generator::holding(
                            id,
                            "agent-00".into(),
                            3361,
                            claim.clone(),
                        )],
                    )
                })
                .collect();
            black_box(engine.commit_batch(commits).required("batch"))
        })
    });
    group.bench_function("transmit_batch_2000", |b| {
        b.iter(|| {
            sequence += 1;
            let inputs = (0..2000)
                .map(|index| TransmitInput {
                    on: Default::default(),
                    timeline_id: None,
                    project_id: "town".into(),
                    attempt_id: format!("bench-transmit-{sequence}-{index}"),
                    position: Position { tick: 3361 },
                    from: "agent-00".into(),
                    to: generator::agent(1 + index % 39),
                    source_record_id: "initial-0".into(),
                    record_id: Some(format!("bench-transmitted-{sequence}-{index}")),
                    attitude: Attitude::Believes,
                    confidence: 0.7,
                    intent: Intent::Honest,
                    distorted_claim: None,
                    text: None,
                })
                .collect();
            black_box(engine.transmit_batch(inputs).required("transmit batch"))
        })
    });
    group.bench_function("witness_scene_40", |b| {
        b.iter(|| {
            sequence += 1;
            let input = WitnessInput {
                project_id: "town".into(),
                attempt_id: format!("bench-witness-{sequence}"),
                timeline_id: None,
                expected_version: None,
                event: generator::workload_commit(format!("bench-scene-{sequence}"), 3361, vec![])
                    .event,
                claims: (0..3)
                    .map(|index| WitnessedClaim {
                        claim: generator::benchmark_claim(index, index + 1),
                        canon: false,
                        fact_id: None,
                    })
                    .collect(),
                witnesses: (0..40)
                    .map(|index| Witness {
                        holder: generator::agent(index),
                        attitude: Attitude::Knows,
                        confidence: 1.0,
                        accessibility: Accessibility::Available,
                        record_ids: Vec::new(),
                    })
                    .collect(),
            };
            black_box(engine.witness(input).required("witness"))
        })
    });
    group.bench_function("adjust_batch_2000", |b| {
        // Each night adjusts 2,000 fresh Holdings; creating them is not timed.
        b.iter_custom(|iterations| {
            let mut total = Duration::ZERO;
            for _ in 0..iterations {
                sequence += 1;
                let ids = (0..2000)
                    .map(|index| format!("bench-fade-{sequence}-{index}"))
                    .collect::<Vec<_>>();
                let records = ids
                    .iter()
                    .enumerate()
                    .map(|(index, id)| {
                        generator::holding(
                            id.clone(),
                            generator::agent(index % 40),
                            3361,
                            claim.clone(),
                        )
                    })
                    .collect();
                engine
                    .commit(generator::workload_commit(
                        format!("bench-fade-setup-{sequence}"),
                        3361,
                        records,
                    ))
                    .required("adjust setup");
                let input = AdjustHoldingsInput {
                    on: Default::default(),
                    project_id: "town".into(),
                    attempt_id: format!("bench-adjust-{sequence}"),
                    timeline_id: None,
                    expected_version: None,
                    position: Position { tick: 3362 },
                    adjustments: ids
                        .into_iter()
                        .map(|record_id| HoldingAdjustment {
                            record_id,
                            new_record_id: None,
                            confidence: Some(0.5),
                            accessibility: None,
                            attitude: None,
                        })
                        .collect(),
                    provenance: None,
                };
                let start = std::time::Instant::now();
                black_box(engine.adjust_holdings(input).required("adjust"));
                total += start.elapsed();
            }
            total
        })
    });
    group.bench_function("open", |b| {
        b.iter(|| black_box(SqliteStore::open(&dataset).required("open")))
    });
    group.finish();
    drop(engine);
    let latency_fixture = working_directory.path().join("latency.umw");
    std::fs::copy(&dataset, &latency_fixture).required("copy latency fixture");
    let mut latency_engine = Engine::new(
        SqliteStore::open(&latency_fixture).required("open latency fixture"),
        Arc::new(FixedClock("2026-01-01T00:00:00Z".into())),
        Arc::new(UuidV7Gen),
    );
    record_latencies(&mut latency_engine, &dataset, &root);
}

fn samples(mut operation: impl FnMut(), count: usize) -> serde_json::Value {
    let mut durations = Vec::with_capacity(count);
    for _ in 0..count {
        let start = std::time::Instant::now();
        operation();
        durations.push(start.elapsed().as_nanos() as f64);
    }
    let mean = durations.iter().sum::<f64>() / count as f64;
    durations.sort_by(f64::total_cmp);
    serde_json::json!({"meanNs":mean,"p99Ns":durations[(count * 99).div_ceil(100) - 1],"samples":count})
}

fn record_latencies(
    engine: &mut Engine<SqliteStore>,
    dataset: &std::path::Path,
    root: &std::path::Path,
) {
    let reference = ClaimRef::Claim {
        claim: generator::benchmark_claim(0, 1),
    };
    let holds = samples(
        || {
            black_box(
                engine
                    .holds("town", "agent-00", reference.clone(), At::default())
                    .required("holds"),
            );
        },
        200,
    );
    let holdings = samples(
        || {
            black_box(
                engine
                    .holdings(
                        "town",
                        "agent-00",
                        HoldingFilter {
                            about: Some("agent-01".into()),
                            ..Default::default()
                        },
                        At::default(),
                        Page {
                            limit: 50,
                            cursor: None,
                        },
                    )
                    .required("holdings"),
            );
        },
        200,
    );
    let mut night = 0;
    let transmits = samples(
        || {
            night += 1;
            let inputs = (0..2000)
                .map(|index| TransmitInput {
                    on: Default::default(),
                    timeline_id: None,
                    project_id: "town".into(),
                    attempt_id: format!("latency-night-{night}-{index}"),
                    position: Position { tick: 3361 },
                    from: "agent-00".into(),
                    to: generator::agent(1 + index % 39),
                    source_record_id: "initial-0".into(),
                    record_id: Some(format!("latency-record-{night}-{index}")),
                    attitude: Attitude::Believes,
                    confidence: 0.7,
                    intent: Intent::Honest,
                    distorted_claim: None,
                    text: None,
                })
                .collect();
            black_box(engine.transmit_batch(inputs).required("night"));
        },
        10,
    );
    let open = samples(
        || {
            black_box(SqliteStore::open(dataset).required("open"));
        },
        100,
    );
    let data = serde_json::json!({"holds":holds,"holdings_about_50":holdings,"transmit_batch_2000":transmits,"open":open});
    std::fs::write(
        root.join("target/benchmark-latencies.json"),
        serde_json::to_vec_pretty(&data).required("latency JSON"),
    )
    .required("latency file");
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
