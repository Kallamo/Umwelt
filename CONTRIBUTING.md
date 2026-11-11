# Contributing to Umwelt

Thank you for taking the time to contribute. Umwelt is an open-source engine
built by [Kallamo](https://kallamo.com), and it grows through the people who
use it, report what breaks and improve it. Whether you are fixing a typo,
reporting a bug or adding a feature, your help is welcome.

This document explains how to get involved and what to expect.

## Table of contents

- [Code of conduct](#code-of-conduct)
- [Ways to contribute](#ways-to-contribute)
- [Reporting bugs](#reporting-bugs)
- [Suggesting features](#suggesting-features)
- [Setting up your environment](#setting-up-your-environment)
- [Development workflow](#development-workflow)
- [Validation](#validation)
- [Commit conventions](#commit-conventions)
- [Submitting a pull request](#submitting-a-pull-request)
- [Contributor License Agreement](#contributor-license-agreement)
- [Using AI tools](#using-ai-tools)
- [Architecture](#architecture)
- [Changing the contract](#changing-the-contract)
- [Compatibility](#compatibility)
- [Performance](#performance)
- [Coding guidelines](#coding-guidelines)
- [Releasing](#releasing)
- [Branding and trademark](#branding-and-trademark)
- [Questions](#questions)

## Code of conduct

Everyone who takes part in Umwelt follows the
[Kallamo Code of Conduct](CODE_OF_CONDUCT.md). In short: be respectful, be
constructive, and assume good faith. Harassment, personal attacks or
dismissive behavior toward other contributors are not tolerated. To report
something, write to conduct@kallamo.com or message a moderator on
[Discord](https://discord.com/invite/CE4C9JRS9H).

## Ways to contribute

You do not need to write code to make a difference:

- **Use the engine** and report what breaks or feels rough.
- **Report bugs** with a minimal reproduction.
- **Suggest features** by describing the problem they solve.
- **Improve the documentation**: the guides in `docs/`, the specification in
  `spec/`, the READMEs and the examples.
- **Write code** to fix bugs or add features.

Security vulnerabilities are not reported in public issues: follow
[SECURITY.md](SECURITY.md).

## Reporting bugs

Before opening an issue, [search the existing ones](https://github.com/Kallamo/Umwelt/issues)
to avoid duplicates. A good bug report includes:

- **What happened** and **what you expected**.
- **The version**: the output of `umwelt --version`, or the `engine`,
  `contract` and `storage` fields of the `hello` result.
- **A minimal reproduction**, ideally the sequence of protocol requests that
  shows the problem (the same shape as the cases in `conformance/cases`). A
  small `.umw` file helps too, if it holds nothing private.
- **Your environment**: operating system, and which client you use (CLI,
  `@kallamo/umwelt`, `Kallamo.Umwelt`, or your own over the protocol).

## Suggesting features

Describe **the problem you are trying to solve**, not only the solution you
have in mind. Umwelt keeps a small set of guarantees ([spec/invariants.md](spec/invariants.md))
and offers everything generically: a feature that serves one application must
be expressible as a general capability that any client can use. Opening an
issue, or raising the idea on Discord first, is the best way to start before
investing time in code.

## Setting up your environment

You need:

- **Rust** 1.85 or newer (stable);
- **Node.js** 22 or newer and **npm** 10 or newer;
- the **.NET 6 SDK**, only to work on the .NET binding.

```bash
git clone https://github.com/Kallamo/Umwelt.git
cd umwelt
npm install
cargo build --workspace
```

The repository holds:

| Path | Contents |
| --- | --- |
| `crates/umwelt-core` | the contract and the rules: validation, materialization, claim keys. No I/O. |
| `crates/umwelt-sqlite` | storage: atomic writes, indexed reads, full-text and semantic search. The only code that knows SQL. |
| `crates/umwelt-api` | the public `Engine`: commands and queries, clocks and id generators. |
| `crates/umwelt-protocol` | the JSON protocol: handshake, envelopes, dispatcher, error codes. |
| `crates/umwelt-ffi` | the C ABI over the same protocol. |
| `apps/cli` | the `umwelt` command: create, inspect, `serve`, `call`, benchmark generator. |
| `packages/umwelt` | the TypeScript client (`@kallamo/umwelt`). |
| `bindings/csharp` | the .NET binding (`Kallamo.Umwelt`) and its conformance runner. |
| `spec` | the specification and the generated JSON Schemas. |
| `conformance` | language-neutral cases that every client runs. |
| `docs` | guides. |
| `benchmarks` | methodology and recorded results. |
| `examples` | small programs for each client. |

## Development workflow

1. **Fork** the repository and clone your fork.
2. **Create a branch** off `main`, named by the kind of work: `fix/…`,
   `feat/…`, `docs/…`, `refactor/…`.
3. **Make your change** in focused, logically grouped commits.
4. **Run the validation** below.
5. **Push** to your fork and open a pull request against `main`.

Keep pull requests focused. Several small, self-contained pull requests are
easier to review than one that mixes unrelated changes.

## Validation

Every pull request must pass:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

When the change touches the TypeScript client, the schemas or the protocol:

```bash
npm run schema:check
npm run typecheck
npm test
```

When it touches the schemas or the .NET binding (needs the .NET 6 SDK):

```bash
npm run check:csharp
npm run test:csharp
```

When it adds a dependency:

```bash
cargo deny check licenses
```

## Commit conventions

Umwelt follows [Conventional Commits](https://www.conventionalcommits.org/):

```
type: short summary in the present tense
```

Common types: `fix`, `feat`, `docs`, `refactor`, `perf`, `test`, `chore`,
`release`. Write summaries in English and describe *what* the change does.
The reasoning behind a change goes in the commit body, not in code comments.

## Submitting a pull request

Describe the change and the issue it addresses (`Fixes #123` closes the issue
on merge), how you tested it, and anything a reviewer should look at closely.
Update the specification and the guides in the same pull request when the
behavior changes.

A maintainer will review it. Expect questions or change requests: that is
part of keeping the engine reliable, not a rejection of your work. This is a
young project maintained with limited time, so please be patient.

## Contributor License Agreement

Before a contribution can be merged, you must accept the
[Kallamo Contributor License Agreement](CLA.md). The CLA check on your first
pull request asks you to sign it once; the signature covers every later
contribution to any Kallamo project.

In short: you keep all rights to your work (the license you grant is
non-exclusive), the project may license it under other terms in the future
(for example a commercial license next to the AGPL), and every version
already released under the AGPL stays under the AGPL forever. Read the
[full text](CLA.md) before contributing.

## Using AI tools

You may use AI assistants and code generators. What matters is the
contribution, not the tool that helped write it.

- **You are responsible for everything you submit.** Understand every line,
  and be ready to explain and defend your changes in review. "The tool wrote
  it" is not an answer to a review question.
- **Test it.** The validation above must pass, and the behavior must be
  checked, not assumed.
- **Say so when a tool did a substantial part of the work.** One line in the
  pull request description is enough. It helps the review; it is not held
  against you.
- **Low-effort submissions are closed**: unreviewed generated code, invented
  bug reports, or generic text that does not engage with the project.
- **Respect licenses.** Do not submit code a tool reproduced from another
  project under an incompatible license. The representations in the
  [CLA](CLA.md) apply to tool-assisted contributions like any other.

## Architecture

The layers have boundaries that keep the engine predictable:

- `umwelt-core` performs no I/O, reads no clock, generates no ids and spawns
  no threads. Storage, clocks and ids are supplied to it.
- `umwelt-sqlite` is the only code that knows SQL. Every read applies the
  timeline visibility rule, and every positioned read honors the axis it reads
  on.
- The clients (`@kallamo/umwelt`, `Kallamo.Umwelt`, the CLI) speak the
  protocol and never implement narrative rules.
- No logic specific to one application, game or product enters the engine.
- The engine never infers, never applies policy and never runs or calls a
  model: probabilities, decay, trust and vectors come from the client.

Every change keeps the guarantees in [spec/invariants.md](spec/invariants.md).
The specification in `spec/` describes the behavior that clients rely on; a
change in behavior updates it in the same pull request.

## Changing the contract

The Rust types in `umwelt-core` are the source of the contract. Adding or
changing a field, an operation or an error:

1. Change the Rust types and the behavior.
2. Regenerate the schemas: `npm run schema:generate`.
3. Regenerate the clients: `npm run generate:types` and
   `npm run generate:csharp`.
4. Add at least one conformance case in `scripts/generate-conformance.mjs`
   and regenerate the cases with `node scripts/generate-conformance.mjs`.
5. Update the specification in `spec/`.

The wire format uses camelCase fields and kebab-case enum values. Error
messages are English diagnostics; clients branch on the error `code`.

## Compatibility

Umwelt follows semantic versioning, with one version shared by every package
of a release. [spec/versioning.md](spec/versioning.md) defines what counts as
an incompatible change. In short, these are incompatible: removing or
renaming a field, an operation or an error code, and changing what an existing
field means. These are not: adding an optional field or a new operation,
changing an error message, and changing the order of items the specification
says carry no meaning.

Changing the layout of `.umw` files raises the storage version and comes with
a migration from the previous version, so that existing files keep opening.

## Performance

- Paginated queries read their rows in index order. The query-plan tests in
  `crates/umwelt-sqlite/tests` reject a temporary sort (`TEMP B-TREE`) in them;
  a new paginated query gets a plan test.
- A lookup through a partial index names it (`INDEXED BY`) and gets a plan
  test.
- A change to a read or write path should be checked against the benchmarks
  ([benchmarks/README.md](benchmarks/README.md)) before and after.

## Coding guidelines

- **Match the surrounding code.** Follow the structure, naming and style of
  the file you are editing.
- **Write in English**: code, comments, commit messages and documentation.
- **Start every source file with its license header**:
  `// SPDX-License-Identifier: AGPL-3.0-only` (`-- ` in SQL, `# ` in TOML).
- **No `unwrap` or `expect` outside tests** in the core, API and FFI crates.
- **Errors** use `thiserror` and carry a stable public code.
- **Keep comments short and rare.** A comment exists only for what the code
  cannot say: a non-obvious *why*, an invariant someone could break, or a
  trap.
  - Do not narrate history ("used to", "now", "this fixes", "previously").
    That belongs in the commit message.
  - Do not describe callers, or what the signature and names already show.
  - Do not restate the code in prose.
- **New dependencies** must have a license accepted by `deny.toml`.

## Releasing

Maintainers release from a version tag:

1. Set the new version in `Cargo.toml` (workspace), `packages/umwelt/package.json`
   and `bindings/csharp/Kallamo.Umwelt/Kallamo.Umwelt.csproj`;
   `npm run check:versions` confirms they agree.
2. Date its section in `CHANGELOG.md`.
3. Push the tag `vX.Y.Z`. The release workflow checks everything again, builds
   every platform, and publishes the npm packages, the NuGet package and the
   GitHub release with the executables, native libraries, C header and
   third-party notices.

## Branding and trademark

The source code is licensed under the [GNU AGPLv3](LICENSE), but the
**Umwelt** and **Kallamo** names and logos are trademarks of the project's
creator. This does not affect contributing. It means that an independent fork
or redistribution must use its own name and branding.

## Questions

If anything here is unclear, or you want to talk an idea through before
opening an issue, join us on [Discord](https://discord.com/invite/CE4C9JRS9H).

Thank you for contributing to Umwelt.
