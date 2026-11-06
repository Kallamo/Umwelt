# Public Engine API

The Rust API is `umwelt_api::Engine<S: Store>`, built from a store, a `Clock`
and an `IdGen`. The crates are not published on crates.io yet; depend on them
from the Git repository (`https://github.com/Kallamo/Umwelt`).

```rust
let engine = Engine::new(
    SqliteStore::open("story.umw")?,
    Arc::new(SystemClock),
    Arc::new(UuidV7Gen),
);
```

Every protocol operation maps to one `Engine` method, and the data contract is
`umwelt_core::domain`, re-exported by `umwelt-api`. The JSON Schemas in
`spec/schema/` are generated from the same Rust types and are authoritative for
every other language.

- **Commands** take an input struct and return the stored object or a
  `CommitOutcome`: `commit`, `commit_batch`, `transmit`, `witness`,
  `adjust_holdings`, `transition_batch`, `set_dispositions`, `ensure_entities`,
  `define_axis`, the embedding operations (`define_embedding_space`,
  `drop_embedding_space`, `put_embeddings`, `delete_embeddings`),
  the proposal workflow (`propose`, `review`, `apply`, `reject`,
  `rebase_proposal`) and the draft workflow (`open_draft`, `promote_draft`,
  `rebase_draft`, `discard_draft`, `purge_draft`).
- **Queries** on `Engine` read main. `engine.in_timeline(id)` returns a
  `TimelineReader` with the same queries for a draft: `holds`, `holdings`,
  `who_holds`, `group_coverage`, `canon`, `perspective`, `compare` and
  `compare_across`, `contradictions`, `beliefs_about`, `breaches`,
  `entity_at`, `entity_known_by`, `check_utterance`, `context_admit`,
  `trace`, `timeline`, `record`, `search` and `search_with` (semantic and
  hybrid), and `pending_embeddings`.
- Every query takes an `At`: a position (`tick`), optionally on an additional
  axis (`axis`), and a version (`as_of_version`).

Errors are `UmweltError` values with a stable `UmweltErrorCode`; the protocol
maps them to the codes in `spec/errors.md`.
