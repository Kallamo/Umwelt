<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Versioning

Umwelt has one product version and three format versions. The product version
tells you whether your code keeps working; the format versions tell you
whether a file, a client or a native library is compatible.

| Version | Where it appears | Current |
| --- | --- | --- |
| Product | the CLI, `@kallamo/umwelt`, `Kallamo.Umwelt`, the crates, release tags, `hello.engine` | `0.1.0` |
| Contract | `hello.contract`, the JSON Schemas | the product version |
| Protocol | `v` in every envelope, `hello.protocol` | `1` |
| Storage | inside every `.umw` file, `hello.storage` | `5` |
| C ABI | `umwelt_abi_version()` | `1` |

## Product and contract

The product follows [semantic versioning](https://semver.org). Every package
of a release is published from the same commit with the same version: a
`0.3.2` CLI, `@kallamo/umwelt` `0.3.2` and `Kallamo.Umwelt` `0.3.2` belong
together. The contract, the shape of every payload and result, carries the
same version, so `hello.contract` tells a client exactly which release it is
talking to.

| Change | Before 1.0 | From 1.0 |
| --- | --- | --- |
| A fix that changes neither the contract nor the files | patch: `0.1.0` → `0.1.1` | patch: `1.2.0` → `1.2.1` |
| An addition: a new optional field, operation or error code | minor: `0.1.0` → `0.2.0` | minor: `1.2.0` → `1.3.0` |
| An incompatible change | minor: `0.1.0` → `0.2.0` | major: `1.2.0` → `2.0.0` |

Before 1.0 a minor version may contain incompatible changes; the changelog
names every one of them.

### What is an incompatible change

- removing or renaming a field, an operation, an enum value or an error code;
- making an optional field required, or narrowing the values a field accepts;
- changing what an existing field, operation or error code means;
- changing the guarantees in [invariants.md](invariants.md).

### What is not

- adding an optional field, an operation, an enum value or an error code
  (clients must ignore result fields they do not know);
- changing an error `message` (clients branch on `code`);
- changing the order of items the specification says carries no meaning;
- performance changes, and changes to the storage layout that a migration
  covers (see below).

## Protocol

The protocol version covers the envelope only: `v`, `id`, `op`, `payload`,
`ok`, `result` and `error`, and the `hello` handshake. Operations and fields
are part of the contract, not of the protocol, so adding them never changes
the protocol version. A new protocol version would be negotiated in `hello`,
where a client states the range it supports.

## Storage

Every `.umw` file records the storage version that wrote it. The engine reads
and writes files of its own storage version.

When a release changes the file layout, the storage version goes up and the
release migrates files of the previous versions automatically: on open, the
engine copies the file to a backup next to it (`story.umw.v5.bak`), converts
it, and continues. A file of a newer storage version than the engine knows is
refused with `UNSUPPORTED_FILE_VERSION`: upgrade the engine to open it.

## C ABI

The C ABI version covers the native functions and their signatures. The .NET
binding checks it when it loads the native library and refuses a library of
another ABI version.
