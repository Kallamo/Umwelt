// SPDX-License-Identifier: AGPL-3.0-only
// Every package of a release shares one version (spec/versioning.md).
//
//   node scripts/check-versions.mjs [--tag v1.2.3]
//
// Fails when the Cargo workspace, the npm package and the NuGet package
// disagree, or when they do not match the given release tag.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const read = path => readFileSync(resolve(root, path), 'utf8');

const versions = {
  'Cargo.toml (workspace)': read('Cargo.toml').match(/^\[workspace\.package\][^[]*?^version = "([^"]+)"/ms)?.[1],
  'packages/umwelt/package.json': JSON.parse(read('packages/umwelt/package.json')).version,
  'bindings/csharp/Kallamo.Umwelt/Kallamo.Umwelt.csproj': read('bindings/csharp/Kallamo.Umwelt/Kallamo.Umwelt.csproj').match(/<Version>([^<]+)<\/Version>/)?.[1],
};

const tagIndex = process.argv.indexOf('--tag');
const tag = tagIndex >= 0 ? process.argv[tagIndex + 1] : undefined;
const expected = tag ? tag.replace(/^v/, '') : versions['Cargo.toml (workspace)'];

let failed = false;
for (const [source, version] of Object.entries(versions)) {
  if (version !== expected) {
    console.error(`${source}: ${version ?? 'no version found'}, expected ${expected}`);
    failed = true;
  }
}
if (failed) process.exit(1);
console.log(`All packages are at ${expected}.`);
