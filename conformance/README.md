# Shared conformance cases

Both runners execute `cases/*.json` on a fresh engine. Requests omit v/id; runners
add them. Cases send explicit hello (handshake rejection cases first send the
request being rejected). Expectations compare object subsets, array length and
order, and exact scalar values. `$any` matches any present value;
`$capture:name` captures it and `$ref:name` reuses it in later requests or expects.

The numbered cases cover the rules of writing and reading the ledger.
`invariant-*` cover the guarantees in [../spec/invariants.md](../spec/invariants.md)
through public observable behavior. Timeline isolation (9) is covered by the
`timeline-*` cases. The pure-core guarantee (8) is enforced by the Rust crates'
dependencies and tests; no transport can observe whether a library performs
I/O.

Run `cargo test -p umwelt-protocol --test conformance` and `npm test`.
The latter builds the CLI before starting the stdio runner.
