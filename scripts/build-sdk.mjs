// SPDX-License-Identifier: AGPL-3.0-only
import { rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const directory = resolve(import.meta.dirname, '../packages/umwelt');
// Clean only this package's generated output, so removed SDK modules cannot ship.
rmSync(resolve(directory, 'dist'), { recursive: true, force: true });
const compiler = resolve(directory, '../../node_modules/typescript/bin/tsc');
const result = spawnSync(process.execPath, [compiler, '--build', resolve(directory, 'tsconfig.json'), '--force'], { stdio: 'inherit' });
process.exit(result.status ?? 1);
