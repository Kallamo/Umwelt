// SPDX-License-Identifier: AGPL-3.0-only
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { cpus, totalmem, platform, release } from 'node:os';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const option = name => {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
};
// Defaults to the workspace version in Cargo.toml.
const workspaceVersion = readFileSync(resolve(root, 'Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m)?.[1];
const version = option('--version') ?? workspaceVersion;
// The manifest's name. It describes the hardware and never uses the host
// name, which identifies a personal machine.
const osName = { win32: 'windows', darwin: 'macos', linux: 'linux' }[platform()] ?? platform();
const cpuName = (cpus()[0]?.model ?? 'cpu')
  .toLowerCase()
  .replace(/\(r\)|\(tm\)|with radeon graphics|processor|amd|intel|core/g, ' ')
  .trim()
  .split(/\s+/)
  .join('-');
const machine = (option('--machine') ?? `${cpuName}-${osName}`).replace(/[^a-zA-Z0-9_.-]/g, '-');
if (!version) {
  throw new Error('Usage: node scripts/bench-record.mjs [--collect-only] [--version X.Y.Z] [--machine <name>]');
}

if (!process.argv.includes('--collect-only')) {
  const result = spawnSync('cargo', ['bench', '-p', 'umwelt-api', '--bench', 'engine', '--', '--noplot'], { cwd: root, stdio: 'inherit' });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
const names = ['holds', 'holdings_about_50', 'who_holds', 'holds_draft_depth_1', 'holdings_about_50_draft_depth_1', 'holds_draft_depth_3', 'holdings_about_50_draft_depth_3', 'commit', 'commit_batch_100', 'transmit_batch_2000', 'witness_scene_40', 'adjust_batch_2000', 'open'];
function collect(group, benchmarks, required = true) {
  const collected = {};
  for (const name of benchmarks) {
    const path = resolve(root, 'target/criterion', group, name, 'new');
    if (!existsSync(path)) {
      if (required) throw new Error(`Missing benchmark ${group}/${name}`);
      return undefined;
    }
    const estimates = JSON.parse(readFileSync(resolve(path, 'estimates.json'), 'utf8'));
    const sample = JSON.parse(readFileSync(resolve(path, 'sample.json'), 'utf8'));
    const means = sample.times.map((time, index) => time / sample.iters[index]).sort((a, b) => a - b);
    collected[name] = { meanNs: estimates.mean.point_estimate, sampleMeanP99Ns: means[Math.ceil(means.length * 0.99) - 1], samples: means.length };
  }
  return collected;
}
const timings = collect('town', names);
// Optional groups (UMWELT_BENCH_AXES, UMWELT_BENCH_EMBEDDINGS) and their fixtures.
const axisTimings = collect('town-reading', ['holds_main_axis', 'holdings_about_50_main_axis', 'holds_reading_axis', 'holdings_about_50_reading_axis'], false);
const embeddingTimings = collect('town-embeddings', ['semantic_holder', 'semantic_narrator', 'hybrid_holder', 'put_1000'], false);

// File size and per-object storage of the generated fixture (SQLite dbstat).
const binary = resolve(root, 'target/release', platform() === 'win32' ? 'umwelt.exe' : 'umwelt');
const stats = spawnSync(binary, ['bench-stats', resolve(root, 'target/town.umw')], { encoding: 'utf8' });
if (stats.status !== 0) throw new Error(`bench-stats failed: ${stats.stderr}`);
const storage = JSON.parse(stats.stdout);
const fixtureStats = file => {
  const path = resolve(root, 'target', file);
  if (!existsSync(path)) return undefined;
  const result = spawnSync(binary, ['bench-stats', path], { encoding: 'utf8' });
  return result.status === 0 ? JSON.parse(result.stdout) : undefined;
};

const result = {
  version, timestamp: new Date().toISOString(), machine,
  cpu: cpus()[0]?.model, logicalCpus: cpus().length, ramBytes: totalmem(), os: `${platform()} ${release()}`,
  rustc: spawnSync('rustc', ['--version'], { encoding: 'utf8' }).stdout.trim(),
  dataset: { profile: 'town', seed: 42, agents: 40, days: 3360, holdings: 504040 },
  durability: 'normal',
  methodology: 'Criterion release, 10 samples, 500 ms warm-up. sampleMeanP99Ns is a percentile of batch-average durations, not individual-request p99. Individual latencies use a fresh fixture with no separate warm-up: 200 reads, 10 nights, 100 opens.',
  timings,
  individualLatencies: JSON.parse(readFileSync(resolve(root, 'target/benchmark-latencies.json'), 'utf8')),
  storage,
  ...(axisTimings ? { axes: { fixture: 'town --reading-axis', timings: axisTimings, storage: fixtureStats('town-reading.umw') } } : {}),
  ...(embeddingTimings ? { embeddings: { fixture: 'town --embeddings 384', timings: embeddingTimings, storage: fixtureStats('town-embeddings.umw') } } : {}),
};
const directory = resolve(root, 'benchmarks/results', version);
mkdirSync(directory, { recursive: true });
const path = resolve(directory, `${machine}.json`);
writeFileSync(path, JSON.stringify(result, null, 2) + '\n');
console.log(path);
console.table(timings);
if (axisTimings) console.table(axisTimings);
if (embeddingTimings) console.table(embeddingTimings);
console.log(`file: ${(storage.fileBytes / 1048576).toFixed(1)} MiB`);
