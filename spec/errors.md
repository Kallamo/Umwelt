<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Protocol error codes

The following protocol v1 codes are frozen. Existing code text must never be changed or reused for another meaning.

| Code | Meaning |
| --- | --- |
| `VALIDATION_FAILED` | The request or payload is malformed, incomplete, or violates a domain rule. |
| `NOT_FOUND` | The requested Umwelt object or file does not exist. |
| `VERSION_CONFLICT` | `expectedVersion` does not equal the current project version. |
| `CONFLICT` | The request conflicts with already stored state. |
| `INVALID_STATE` | The operation is valid in shape but cannot run in the object's current lifecycle state. |
| `HANDSHAKE_REQUIRED` | A non-`hello` operation was sent before the handshake. |
| `UNSUPPORTED_PROTOCOL` | The requested protocol version or range does not include version 1. |
| `UNKNOWN_OP` | The requested operation name is not part of protocol v1. |
| `FILE_LOCKED` | Another process owns the exclusive lock for the `.umw` file. |
| `NOT_UMWELT_FILE` | The SQLite file does not have the Umwelt application identifier. |
| `UNSUPPORTED_FILE_VERSION` | The file storage version is newer than this engine supports. |
| `PAYLOAD_TOO_LARGE` | A request line exceeds 16 MiB. |
| `INTERNAL` | An unexpected engine, storage, serialization, or I/O failure occurred. |

Error messages are English diagnostics for humans. Programs must branch on `code`, not `message`.
