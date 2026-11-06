# Security policy

## Reporting a vulnerability

Please report security vulnerabilities privately, by e-mail to
**security@kallamo.com**. Do not open a public issue, pull request or Discord
message for a vulnerability until it is fixed.

Include, as far as you can:

- the affected component (engine, CLI, `@kallamo/umwelt`, `Kallamo.Umwelt`, C
  ABI) and version (`umwelt --version`, or the `engine`, `contract` and
  `storage` fields of the `hello` result);
- a description of the issue and its impact;
- the steps, requests or file needed to reproduce it;
- any suggested fix.

## What to expect

Umwelt is maintained by a small team, so these are targets, not guarantees:

- an acknowledgement within 7 days;
- an assessment and, if the report is confirmed, a plan for a fix within 30
  days;
- a coordinated disclosure: the fix is released first, then the issue is
  published with credit to the reporter, unless you prefer to stay anonymous.

## Supported versions

Before 1.0, security fixes are released for the latest minor version only
(for example `0.3.x` once `0.3.0` is out). Upgrade to receive them.

## Scope

In scope:

- the engine and its storage: a crafted `.umw` file, request or sequence of
  requests that corrupts data, crashes the process, exhausts resources out of
  proportion to the input, or reads or writes outside what the request allows
  (another project, another timeline, a record the read cannot see);
- the protocol server (`umwelt serve`), the C ABI and the client libraries;
- the published packages and release artifacts.

Out of scope: vulnerabilities in applications built on Umwelt, unless the
engine itself causes them, and attacks that require already controlling the
process or the machine that runs the engine.
