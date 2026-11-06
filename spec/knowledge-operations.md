<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Knowledge operations

Operations added on top of the v1 core: external entities, witnesses, groups,
batch maintenance, names over time, second-order knowledge, secrets,
continuity checks, context admission and rebase. Payload and result shapes are
in `schema/umwelt-protocol-v1.schema.json` (`x-ops`). Every write accepts
`timelineId` and `expectedVersion`; every read accepts `timelineId` (see
[timelines.md](timelines.md)).

None of these operations applies policy. How much a memory fades, who trusts
whom, who may hear a secret and which line of dialogue is acceptable are the
client's decisions; the engine records them and reports what follows.

## External entities

| Operation | Payload | Result |
| --- | --- | --- |
| `entity.ensureBatch` | `{ projectId, entities: [{ externalRef, id?, typeId, name, aliases?, fields?, createdAt? }] }` | `[{ entity, created }]` |
| `entity.resolve` | `{ projectId, externalRef }` | `Entity` |
| `entity.resolveBatch` | `{ projectId, externalRefs }` | `[{ externalRef, entityId? }]` |

`externalRef` is `{ system, id }`, unique per project. `ensureBatch` creates
missing entities and updates the name, aliases and fields of existing ones, in
one transaction. Ensuring the same input twice changes nothing. An existing
entity keeps its id and type: a different `typeId` is `VALIDATION_FAILED`, a
different `id` is `CONFLICT`, and the same reference twice in one batch is
`VALIDATION_FAILED`.

Other operations take engine entity ids. A client either keeps the map that
`ensureBatch` returns, or chooses the entity ids itself (for example the
game's own ids) when it ensures them.

## Witnesses

`witness { projectId, attemptId, event, claims: [{ claim, canon?, factId? }], witnesses: [{ holder, attitude?, confidence?, accessibility?, recordIds? }] }`
returns a `CommitOutcome`.

One commit records the event and, for every witness and every claim, a Holding
acquired as `witnessed` at that event. Defaults: attitude `knows`, confidence
1, accessibility `available`. A claim with `canon: true` is also recorded as a
Fact. `recordIds` gives one Holding id per claim. The attempt fingerprint is
computed from the request, so a retry is recognized even when the event and
record ids were generated.

## Groups

- `whoHolds` accepts `filter.memberOf`: only holders that are members of the
  group at the read position (an active `umwelt:member-of` Fact).
- `groupCoverage { projectId, group, claim | claimKey, at }` returns the
  members, the members holding the claim, the members holding nothing about it,
  and the group's own Holding if the group itself holds it. Members knowing
  something is never the group knowing it.

## Batch maintenance

| Operation | Effect |
| --- | --- |
| `holding.adjustBatch` | Each adjustment supersedes an active Holding with a copy carrying a new `confidence`, `accessibility` and/or `attitude` (optionally with `newRecordId`). |
| `transition.batch` | One status (`retracted`, `revoked`, `forgotten`, `abandoned`) for many records. `superseded` is refused: use `holding.adjustBatch`. |
| `disposition.setBatch` | Many Disposition values (`holder`, `target`, `dimension`, `value`). The newest per holder, target and dimension is the current one. |

Each is one commit with one engine event of type `umwelt:adjust`,
`umwelt:transition` or `umwelt:disposition`, kept out of full-text search.
History stays readable: a read before the batch sees the old values.

## Names over time

Reserved predicates with string values:

- `umwelt:name` (one per subject at a time);
- `umwelt:title` (one per subject at a time);
- `umwelt:alias` (many).

`entity.at { entityId, at }` returns the name, title and aliases canon gives
the entity at that position, falling back to the entity's own name. The latest
active Fact wins; a negated latest Fact clears the title.

`entity.asKnownBy { entityId, holder, at }` returns only what the holder's
active, available Holdings say, plus whether the holder has an Awareness record
of the entity. A holder that was never told a name gets none.

## Second-order knowledge

`umwelt:holds` is a reserved predicate whose subject is a holder (an agent or
group) and whose value is `{ "claimKey": "<64 hex>" }`. A Holding of Alice with
subject Bob and this predicate means "Alice believes Bob holds that claim"; a
negated claim means "Alice believes Bob does not".

`beliefsAbout { holder, about, at }` lists the holder's `umwelt:holds`
Holdings about `about`, each with `actuallyHolds` and `actualRecordId`: what
`about` really holds at the same position.

## Secrets

A Holding may carry `secrecy: { sharedWith: [...] }`: its holder keeps it
secret and may share it only with those entities. Telling it is never blocked:

- a commit (or `transmit`) that records a Holding told from a secret, for a
  recipient outside `sharedWith` other than the secret's holder, succeeds and
  returns the warning `SECRET_BREACH`;
- `breaches { owner?, at }` lists every such telling up to the read position:
  the secret, its owner, who told, who heard, the recipient's Holding, its
  position and event. Retelling by someone who heard it second-hand is reached
  through `trace`.

## Continuity check

`check.utterance { holder, at, claims }` checks claims a holder says or thinks.
The claims come from the client or a model it runs; the engine never reads
free text. For each claim:

| `verdict` | Meaning |
| --- | --- |
| `holds` | The holder holds it, with the same polarity (`attitude`, `confidence`). |
| `holds-opposite` | The holder holds it with the opposite polarity. |
| `holds-other-value` | The holder holds another value in the same functional slot. |
| `withheld` | The holder holds it, but suppressed or inaccessible. |
| `future` | The holder only comes to hold it at a later tick: an anachronism. |
| `unknown` | Nothing the holder holds covers it. |

`canon` is reported separately as `agrees`, `contradicts` or `no-canon`, with
the Fact behind it. Canon being true never makes the holder know it.
`consistent` is true when every claim `holds`.

## Context admission

`context.admit { holder?, at, candidates, budget?, excludeClassifications?, includeWithheld?, authorizedNarrator? }`.
Each candidate has the client's `id`, exactly one of `recordId` and
`eventId`, a `cost` (default 1), a `priority` (default 0) and an optional
`classification`. Without `holder` the perspective is the narrator's, who sees
everything visible at the position.

Checks run in this order and a refused candidate reports the first it fails:

1. `invalid-candidate`: neither or both of `recordId` and `eventId`;
2. `not-in-project`: no such record or event in this project and timeline;
3. `future`: recorded after the read position;
4. `inactive`: retracted, revoked, forgotten, abandoned or superseded;
5. `not-held`: outside the holder's perspective. Facts never are in it;
   records of other holders are not; an event is only if the holder has a
   Holding witnessed there (being present grants nothing);
6. `withheld`: a suppressed or inaccessible Holding, unless `includeWithheld`
   with `authorizedNarrator` (asking without authorization is
   `VALIDATION_FAILED`);
7. `excluded-classification`;
8. `duplicate`: the same record or event was already accepted, in input order;
9. `over-budget`: survivors are taken by priority (ties in input order) while
   they fit the budget; a smaller later candidate may still fit.

Admitted candidates are returned highest priority first, refused ones in input
order, with `usedBudget`. Rendering the context is the client's work.

## Rebase

- `proposal.rebase { proposalId }` checks every operation of a pending
  proposal against the current head of its timeline, each together with the
  earlier operations that are still valid, and returns `{ valid, error }` per
  operation. Nothing is applied. A proposal with an `expectedVersion` is moved
  to the current head, so it can be reviewed and applied there.
- `timeline.rebase { projectId, timelineId }` replays an active draft on top
  of main's current head. See [timelines.md](timelines.md#rebase).
