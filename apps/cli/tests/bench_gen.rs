// SPDX-License-Identifier: AGPL-3.0-only
use std::{process::Command, sync::Arc};

use sha2::{Digest, Sha256};
use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore, Store, TimelineScope, canonical_json};

fn logical_hash(path: &std::path::Path) -> String {
    let engine = Engine::new(
        SqliteStore::open(path).expect("open"),
        Arc::new(FixedClock(String::new())),
        Arc::new(SeqIdGen::new()),
    );
    let (version, tick) = engine.store().project_window("town").expect("window");
    let mut records = engine
        .store()
        .records_at("town", &TimelineScope::main(), None, tick, version)
        .expect("records")
        .into_iter()
        .map(|state| serde_json::to_value(state.record).expect("record JSON"))
        .collect::<Vec<_>>();
    records.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    format!(
        "{:x}",
        Sha256::digest(canonical_json(&serde_json::Value::Array(records)).as_bytes())
    )
}

#[test]
fn tiny_profile_has_deterministic_logical_content() {
    let directory = tempfile::tempdir().expect("directory");
    let mut hashes = Vec::new();
    for (index, seed) in [42, 42, 43].into_iter().enumerate() {
        let path = directory.path().join(format!("town-{index}.umw"));
        assert!(
            Command::new(env!("CARGO_BIN_EXE_umwelt"))
                .args([
                    "bench-gen",
                    "--profile",
                    "tiny",
                    "--seed",
                    &seed.to_string()
                ])
                .arg(&path)
                .status()
                .expect("generator")
                .success()
        );
        hashes.push(logical_hash(&path));
    }
    assert_eq!(hashes[0], hashes[1]);
    assert_ne!(hashes[0], hashes[2]);
}
