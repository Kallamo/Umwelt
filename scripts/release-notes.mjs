// SPDX-License-Identifier: AGPL-3.0-only
// Prints the CHANGELOG.md section of one release, for its GitHub release.
//
//   node scripts/release-notes.mjs 1.2.3
//
// Fails when the section is missing or still marked "Unreleased": date the
// release in CHANGELOG.md before tagging it.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const version = process.argv[2]?.replace(/^v/, '');
if (!version) throw new Error('Usage: node scripts/release-notes.mjs <version>');
const changelog = readFileSync(resolve(import.meta.dirname, '../CHANGELOG.md'), 'utf8');
const heading = new RegExp(`^## \\[${version.replaceAll('.', '\\.')}\\] - (.+)$`, 'm');
const match = heading.exec(changelog);
if (!match) throw new Error(`CHANGELOG.md has no section for ${version}.`);
if (/unreleased/i.test(match[1])) throw new Error(`CHANGELOG.md still marks ${version} as unreleased.`);
const start = match.index + match[0].length;
const next = changelog.indexOf('\n## [', start);
process.stdout.write(changelog.slice(start, next === -1 ? undefined : next).trim() + '\n');
