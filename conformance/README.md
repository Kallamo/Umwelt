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

## Write factories

`builders.json` keeps the client libraries' write factories (`claim.entity`,
`knows`, `witnessed`, ...) identical across languages. Each line is a call
with named arguments and the JSON it must build:

```json
{ "call": "claim.entity", "args": { "subject": "bob", "predicate": "stole", "objectId": "necklace" },
  "expect": { "subject": "bob", "predicate": "stole", "object": { "kind": "entity", "id": "necklace" }, "polarity": true } }
```

`packages/umwelt/tests/build.test.ts` and the .NET runner
(`npm run test:csharp`) run every line, and fail when a factory has no line or
a line names a factory the language does not have. A new factory therefore
needs a line here and an implementation in both clients.
