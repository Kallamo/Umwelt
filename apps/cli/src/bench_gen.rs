// SPDX-License-Identifier: AGPL-3.0-only
//! Deterministic client-side workload policy. This is not engine behavior.
use std::{collections::BTreeMap, path::Path, sync::Arc};

use umwelt_api::*;

pub struct SplitMix64(u64);
impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

pub fn provenance() -> Provenance {
    Provenance {
        origin: Origin::System {
            name: "benchmark-generator".into(),
        },
        reason: None,
        metadata: None,
    }
}
pub fn agent(index: usize) -> String {
    format!("agent-{index:02}")
}
pub fn benchmark_claim(subject: usize, object: usize) -> Claim {
    Claim {
        subject: agent(subject),
        predicate: "gift".into(),
        object: ClaimObject::Entity { id: agent(object) },
        qualifiers: BTreeMap::new(),
        polarity: true,
        about: None,
    }
}
pub fn holding(id: String, holder: String, tick: i64, claim: Claim) -> RecordInput {
    RecordInput::Holding {
        common: RecordInputCommon {
            id: Some(id),
            position: Some(Position { tick }),
            provenance: Some(provenance()),
        },
        holder,
        attitude: Attitude::Believes,
        claim,
        acquisition: Acquisition::Authored,
        confidence: 0.8,
        accessibility: Accessibility::Available,
        text: Some("A gift was exchanged in town.".into()),
        secrecy: None,
    }
}
pub fn workload_commit(id: String, tick: i64, records: Vec<RecordInput>) -> CommitInput {
    CommitInput {
        timeline_id: None,
        project_id: "town".into(),
        attempt_id: id.clone(),
        expected_version: None,
        event: EventInput {
            on: Default::default(),
            id: Some(format!("event-{id}")),
            event_type: "town:day".into(),
            title: "Town day".into(),
            description: None,
            position: Position { tick },
            participants: Vec::new(),
            location: None,
            evidence: Vec::new(),
            provenance: provenance(),
        },
        records,
        transitions: Vec::new(),
        preconditions: Vec::new(),
    }
}

/// The additional axis the generator can place every event on.
pub const READING_AXIS: &str = "reading";

/// The embedding space the generator fills with `embeddings`.
pub const EMBEDDING_SPACE: &str = "bench";

/// What the generator adds to the plain workload.
#[derive(Debug, Clone, Copy, Default)]
pub struct Extras {
    /// Define the `reading` axis and give every event a position on it equal
    /// to its day, so reads on that axis see the same rows as on the main
    /// axis.
    pub reading_axis: bool,
    /// Define the `bench` space with this many dimensions and give every
    /// searchable document a pseudo-random vector (its own SplitMix64
    /// stream, so the logical content of the ledger does not change).
    pub embeddings: Option<u32>,
}

/// Stores a pseudo-random vector for every document that has none yet,
/// starting after `resume`.
fn embed_pending(
    engine: &mut Engine<SqliteStore>,
    dimensions: u32,
    random: &mut SplitMix64,
    resume: &mut Option<String>,
) -> umwelt_core::Result<()> {
    loop {
        let page = engine.pending_embeddings(&PendingEmbeddingsInput {
            project_id: "town".into(),
            space_id: EMBEDDING_SPACE.into(),
            timeline_id: None,
            doc_types: Vec::new(),
            page: Page {
                limit: 1000,
                cursor: resume.clone(),
            },
        })?;
        if !page.items.is_empty() {
            let items = page
                .items
                .into_iter()
                .map(|item| EmbeddingItem {
                    vector: (0..dimensions)
                        .map(|_| (random.next() as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32)
                        .collect(),
                    doc_type: item.doc_type,
                    doc_id: item.doc_id,
                    text_hash: item.text_hash,
                })
                .collect();
            engine.put_embeddings(PutEmbeddingsInput {
                project_id: "town".into(),
                space_id: EMBEDDING_SPACE.into(),
                items,
            })?;
        }
        *resume = Some(page.resume_cursor);
        if page.next_cursor.is_none() {
            return Ok(());
        }
    }
}

pub fn generate_with(path: &Path, profile: &str, seed: u64, extras: Extras) -> Result<(), String> {
    let reading_axis = extras.reading_axis;
    let on = |day: i64| {
        if reading_axis {
            AxisPositions::from([(READING_AXIS.to_owned(), day)])
        } else {
            AxisPositions::new()
        }
    };
    let (holders, days, authored, gossip) = match profile {
        "town" => (40_usize, 3360_i64, 100_usize, 50_usize),
        "tiny" => (4, 3, 8, 4),
        _ => return Err("Unknown benchmark profile; use town or tiny.".into()),
    };
    let store = SqliteStore::create(path).map_err(|e| e.to_string())?;
    let mut engine = Engine::new(
        store,
        Arc::new(FixedClock("2026-01-01T00:00:00Z".into())),
        Arc::new(SeqIdGen::new()),
    );
    let result = (|| -> umwelt_core::Result<()> {
        engine.create_project(CreateProjectInput {
            id: Some("town".into()),
            name: "Town".into(),
            description: None,
            timescale: "day".into(),
            vocabulary_mode: VocabularyMode::Strict,
        })?;
        if let Some(dimensions) = extras.embeddings {
            engine.define_embedding_space(DefineEmbeddingSpaceInput {
                project_id: "town".into(),
                space: EmbeddingSpace {
                    id: EMBEDDING_SPACE.into(),
                    dimensions,
                    model: "pseudo-random".into(),
                },
            })?;
        }
        if reading_axis {
            engine.define_axis(DefineAxisInput {
                project_id: "town".into(),
                axis: AxisDef {
                    id: READING_AXIS.into(),
                    label: "Reading order".into(),
                },
            })?;
        }
        engine.create_entity_type(CreateEntityTypeInput {
            project_id: "town".into(),
            id: Some("agent".into()),
            name: "Agent".into(),
            kind: EntityKind::Agent,
            fields: Vec::new(),
        })?;
        for index in 0..holders {
            engine.create_entity(CreateEntityInput {
                project_id: "town".into(),
                id: Some(agent(index)),
                type_id: "agent".into(),
                name: format!("Agent {index}"),
                aliases: Vec::new(),
                fields: Default::default(),
                external_ref: None,
                created_at: Position { tick: 0 },
            })?;
        }
        for (id, cardinality) in [
            ("gift", Cardinality::Many),
            ("visits", Cardinality::Many),
            ("resides", Cardinality::One),
        ] {
            engine.define_predicate(DefinePredicateInput {
                project_id: "town".into(),
                predicate: PredicateDef {
                    id: id.into(),
                    label: id.into(),
                    object: ObjectKind::Entity,
                    cardinality,
                    subject_types: None,
                },
            })?;
        }
        let mut sources = (0..holders)
            .map(|index| (agent(index), format!("initial-{index}")))
            .collect::<Vec<_>>();
        let mut initial = (0..holders)
            .map(|index| {
                holding(
                    format!("initial-{index}"),
                    agent(index),
                    0,
                    benchmark_claim(index, (index + 1) % holders),
                )
            })
            .collect::<Vec<_>>();
        for index in 0..holders / 2 {
            initial.push(RecordInput::Fact {
                common: RecordInputCommon {
                    id: Some(format!("fact-{index}")),
                    position: Some(Position { tick: 0 }),
                    provenance: Some(provenance()),
                },
                claim: benchmark_claim(index, (index + 1) % holders),
            });
        }
        let mut first = workload_commit("initial".into(), 0, initial);
        first.event.on = on(0);
        engine.commit_batch(vec![first])?;
        let mut vectors = SplitMix64::new(seed ^ 0x9E37_79B9_7F4A_7C15);
        let mut resume = None;
        if let Some(dimensions) = extras.embeddings {
            embed_pending(&mut engine, dimensions, &mut vectors, &mut resume)?;
        }
        let mut random = SplitMix64::new(seed);
        for day in 1..=days {
            let mut records = Vec::with_capacity(authored);
            for index in 0..authored {
                let holder = (random.next() % holders as u64) as usize;
                let subject = (random.next() % holders as u64) as usize;
                let object = (random.next() % holders as u64) as usize;
                let id = format!("authored-{day:04}-{index:03}");
                records.push(holding(
                    id.clone(),
                    agent(holder),
                    day,
                    benchmark_claim(subject, object),
                ));
                sources[holder] = (agent(holder), id);
            }
            let mut daily = workload_commit(format!("day-{day}"), day, records);
            daily.event.on = on(day);
            engine.commit_batch(vec![daily])?;
            if let Some(dimensions) = extras.embeddings {
                embed_pending(&mut engine, dimensions, &mut vectors, &mut resume)?;
            }
            let mut transmissions = Vec::with_capacity(gossip);
            for index in 0..gossip {
                let from = (random.next() % holders as u64) as usize;
                let to = (from + 1 + (random.next() % (holders - 1) as u64) as usize) % holders;
                let record_id = format!("gossip-{day:04}-{index:03}");
                transmissions.push(TransmitInput {
                    on: on(day),
                    timeline_id: None,
                    project_id: "town".into(),
                    attempt_id: format!("transmit-{day}-{index}"),
                    position: Position { tick: day },
                    from: agent(from),
                    to: agent(to),
                    source_record_id: sources[from].1.clone(),
                    record_id: Some(record_id.clone()),
                    attitude: Attitude::Believes,
                    confidence: 0.7,
                    intent: Intent::Honest,
                    distorted_claim: None,
                    text: None,
                });
                sources[to] = (agent(to), record_id);
            }
            engine.transmit_batch(transmissions)?;
            if day % 100 == 0 {
                eprintln!("Generated {day}/{days} days");
            }
        }
        engine.checkpoint()?;
        engine.store_mut().vacuum()
    })();
    result.map_err(|e| e.to_string())
}

pub fn command(arguments: &[String]) -> Result<i32, String> {
    let mut profile = "town";
    let mut seed = 42;
    let mut extras = Extras::default();
    let mut output = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--profile" => {
                index += 1;
                profile = arguments.get(index).ok_or("Missing profile.")?;
            }
            "--seed" => {
                index += 1;
                seed = arguments
                    .get(index)
                    .ok_or("Missing seed.")?
                    .parse()
                    .map_err(|_| "Invalid seed.")?;
            }
            "--reading-axis" => extras.reading_axis = true,
            "--embeddings" => {
                index += 1;
                extras.embeddings = Some(
                    arguments
                        .get(index)
                        .ok_or("Missing embedding dimensions.")?
                        .parse()
                        .map_err(|_| "Invalid embedding dimensions.")?,
                );
            }
            value if value.starts_with('-') => return Err(format!("Unknown option {value}.")),
            value => {
                if output.replace(value).is_some() {
                    return Err("Only one output file is allowed.".into());
                }
            }
        }
        index += 1;
    }
    generate_with(
        Path::new(output.ok_or(
            "Usage: umwelt bench-gen --profile town --seed 42 [--reading-axis] \
             [--embeddings <dimensions>] <out.umw>",
        )?),
        profile,
        seed,
        extras,
    )?;
    Ok(0)
}

/// `umwelt bench-stats <file.umw>`: file size and bytes per table and index,
/// as JSON, for the benchmark manifest.
pub fn stats_command(arguments: &[String]) -> Result<i32, String> {
    let [path] = arguments else {
        return Err("Usage: umwelt bench-stats <file.umw>".into());
    };
    let file_bytes = std::fs::metadata(path)
        .map_err(|error| format!("Cannot read {path}: {error}"))?
        .len();
    let store = SqliteStore::open(path).map_err(|error| error.to_string())?;
    let breakdown = store
        .storage_breakdown()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|(name, bytes)| serde_json::json!({ "name": name, "bytes": bytes }))
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::json!({ "fileBytes": file_bytes, "objects": breakdown })
    );
    Ok(0)
}
