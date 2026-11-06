// SPDX-License-Identifier: AGPL-3.0-only
// Builds the Kallamo.Umwelt NuGet package with the native libraries that exist.
//
//   node scripts/pack-csharp.mjs [--native <rid>=<path>]... [--output <dir>]
//
// Without --native, the release library of this machine is used
// (target/release). CI passes one --native per runtime it built.
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { arch, platform } from 'node:os';
import { basename, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const project = resolve(root, 'bindings/csharp/Kallamo.Umwelt');
const runtimes = resolve(project, 'runtimes');

function option(name) {
  const values = [];
  for (let index = 0; index < process.argv.length; index++) {
    if (process.argv[index] === name) values.push(process.argv[index + 1]);
  }
  return values;
}

function localRuntime() {
  const cpu = arch() === 'arm64' ? 'arm64' : 'x64';
  switch (platform()) {
    case 'win32': return [`win-${cpu}`, resolve(root, 'target/release/umwelt_ffi.dll')];
    case 'darwin': return [`osx-${cpu}`, resolve(root, 'target/release/libumwelt_ffi.dylib')];
    default: return [`linux-${cpu}`, resolve(root, 'target/release/libumwelt_ffi.so')];
  }
}

const natives = option('--native').map(value => value.split('='));
if (natives.length === 0) {
  const built = spawnSync('cargo', ['build', '-p', 'umwelt-ffi', '--release'], { cwd: root, stdio: 'inherit' });
  if (built.status !== 0) process.exit(built.status ?? 1);
  natives.push(localRuntime());
}

rmSync(runtimes, { recursive: true, force: true });
for (const [rid, path] of natives) {
  if (!existsSync(path)) throw new Error(`Native library for ${rid} not found: ${path}`);
  const directory = resolve(runtimes, rid, 'native');
  mkdirSync(directory, { recursive: true });
  copyFileSync(path, resolve(directory, basename(path)));
  console.log(`${rid}: ${basename(path)}`);
}

const output = option('--output')[0] ?? resolve(root, 'target/nuget');
const packed = spawnSync('dotnet', ['pack', project, '-c', 'Release', '-o', output], { stdio: 'inherit' });
process.exit(packed.status ?? 1);
