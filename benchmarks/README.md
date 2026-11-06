# Benchmarks

```bash
cargo build -p umwelt-cli --release
cargo bench -p umwelt-api --bench engine -- --noplot
node scripts/bench-record.mjs
```

The recording script runs Criterion and writes
`results/<version>/<machine>.json`. The version defaults to the workspace
version (`--version` overrides it); the machine name is derived from the CPU
model and the operating system (`--machine` overrides it) and never uses the
host name. `--collect-only` collects an already completed run. The manifest includes CPU, logical CPUs, RAM, OS, rustc
version, Criterion means and sample-average percentiles, separate
per-operation latency samples, the file size and the bytes used by every table
and index (`umwelt bench-stats <file.umw>`). These are local measurements, not
remote CI results.

## Dataset and reproducibility

`town` contains 40 agents, 3,360 days (tick = day), 504,040 holdings and 20
canonical facts. Three example predicates are defined. Each day adds 100
authored holdings and 50 gossip transmissions through `commitBatch` and
`transmitBatch`. The generator (`umwelt bench-gen --profile town --seed 42
<file.umw>`) uses SplitMix64, a fixed clock and fixed ids, and a fixed
client-side gossip policy; none of that policy lives in the engine. `tiny` is a
quick test profile. The CLI integration test hashes sorted serialized records
and proves that identical seeds produce identical logical content, while
another seed differs. File bytes are not the determinism contract.

Criterion generates `target/town.umw` once if absent, then operates on a
disposable copy. Read measurements precede writes. Writes include payload
construction and validation, use fresh attempt ids, and do not measure
idempotent replays. Individual latencies use another fresh copy, with 200 read
samples, 10 nights and 100 opens. A fixture of another storage version is
refused: delete `target/town*.umw` after a storage change.

Two optional groups use variants of the same workload:

- `UMWELT_BENCH_AXES=1` builds `target/town-reading.umw`
  (`bench-gen --reading-axis`), where every event also sits at its day on a
  `reading` axis, and measures `holds` and `holdings` on both axes (group
  `town-reading`).
- `UMWELT_BENCH_EMBEDDINGS=1` builds `target/town-embeddings.umw`
  (`bench-gen --embeddings 384`), where every searchable document has a
  pseudo-random 384-dimension vector (a separate SplitMix64 stream, so the
  ledger itself is unchanged), and measures semantic search for one holder and
  for the narrator, hybrid search for one holder, and storing 1,000 vectors
  (group `town-embeddings`).

## Targets

| Measure | Target |
| --- | --- |
| holds | p99 < 200 µs |
| holdings (about, 50) | p99 < 2 ms |
| Night of 2,000 transmits | < 500 ms |
| File size of `town` | < 450 MB |

Criterion's `sampleMeanP99Ns` is a percentile of batch averages, not a request
p99. `individualLatencies.p99Ns` uses nearest rank on individual measured
calls; for 10 nights, p99 is the slowest night. These small local samples are
a baseline, not a statistically robust production tail-latency estimate.

## Results

A laptop with an AMD Ryzen 5 5625U, 12 logical CPUs, 15.4 GiB RAM, Windows 11
(10.0.22631), rustc 1.97.1. Release build, seed 42, durability `normal`,
November 5, 2026. Raw manifest:
[0.1.0](results/0.1.0/ryzen-5-5625u-windows.json).
Run-to-run noise on this laptop is about ±10%.

### Criterion (warm cache)

| Operation | Mean | Sample p99 | Target | Met |
| --- | ---: | ---: | --- | --- |
| holds | 55–65 µs | 98 µs | p99 < 200 µs | Yes |
| holdings (about, 50) | 1.35 ms | 1.51 ms | p99 < 2 ms | Yes |
| whoHolds | 2.64 ms | 2.82 ms | — | — |
| Single commit | 1.45 ms | 3.28 ms | — | — |
| commitBatch (100) | 19.3 ms | 24.1 ms | — | — |
| transmitBatch (2,000) | 464 ms | 493 ms | < 500 ms | Yes |
| witness (40 witnesses × 3 claims) | 85 ms | 123 ms | — | — |
| holding.adjustBatch (2,000) | 471 ms | 507 ms | — | — |
| Open file | 4.02 ms | 4.94 ms | — | — |
| File size | 397.9 MiB | — | < 450 MB | Yes |

`holds` measured 64.6 µs in the `town` group and 55.3 µs for the same read in
the `town-reading` group; both are the same query.

The largest objects of the file are `records` (110 MiB), `attempts` (33 MiB)
and the claim indexes (25 to 29 MiB each).

### Individual calls (fresh copy, no warm-up)

| Operation | Mean | p99 |
| --- | ---: | ---: |
| holds | 58.8 µs | 116 µs |
| holdings (about, 50) | 1.36 ms | 2.16 ms |
| transmitBatch (2,000) | 529 ms | 1.31 s |
| Open file | 4.08 ms | 5.14 ms |

The first night pays for reading the file into the page cache, which is the
1.31 s maximum.

### Drafts

The same reads through empty drafts, where every row comes from main through
the draft's ancestry:

| Measure | Main | Draft, depth 1 | Draft, depth 3 |
| --- | ---: | ---: | ---: |
| holds | 55–65 µs | 39 µs | 101 µs |
| holdings (about, 50) | 1.35 ms | 1.80 ms | 1.53 ms |

Draft reads always check transitions instead of the materialized status, and
each level adds one ancestor to the visibility filter.

### Additional time axes (`town-reading`, 409.1 MiB)

| Measure | Main axis | Reading axis |
| --- | ---: | ---: |
| holds | 55 µs | 102 µs |
| holdings (about, 50) | 1.29 ms | 1.67 ms |

A read on an additional axis checks transitions and looks up the event
position of each row and of its transitions: `holds` costs about 1.9 times the
main axis, a holdings page about 1.3 times.

### Semantic search (`town-embeddings`, 1,076.9 MiB)

339,421 vectors of 384 dimensions occupy 665 MiB in the `embeddings` table
(521 MB of raw `f32`).

| Measure | Mean | Sample p99 |
| --- | ---: | ---: |
| Semantic search, one holder (8,325 vectors), page of 20 | 88 ms | 106 ms |
| Semantic search, narrator (every Fact and event vector) | 865 ms | 893 ms |
| Hybrid search, one holder, text "gift" | 1.11 s | 1.23 s |
| `embedding.put`, 1,000 vectors | 27 ms | 30 ms |

Search is exact, so its cost grows with the number of candidates. For one
holder, about 65 ms is the visibility check of each of the holder's 12,539
Holdings and the rest reading and scoring 8,325 vectors. The fixture gives a
vector to every Holding, far more text than a character in a story keeps;
with hundreds to a few thousand documents per holder a search takes a few
milliseconds. Semantic search is meant for building a model's context, whose
own call takes far longer, not for per-frame game logic.

The hybrid figure comes from its text half: every authored Holding of the
fixture has the same text, so "gift" matches 336,000 documents, all of which
the holder filter checks. A text-only search for such a word costs the same.
