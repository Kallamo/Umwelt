# Concepts

Umwelt records what happened in a story, a game or a simulation, and, kept
apart from that, who knows it, how they came to know it and since when. This
page defines the vocabulary used everywhere else. The exact shapes are in the
[specification](../spec/README.md).

## The file and its projects

A **`.umw` file** is a SQLite database that holds one or more **projects**.
Projects are isolated: nothing in one is visible from another. A project has a
`timescale` (a label for what a tick means: a day, a turn, a scene) and a
**vocabulary mode**: `strict` refuses claims with undefined predicates, `open`
accepts them with an `UNKNOWN_PREDICATE` warning.

## Entities

An **entity** is anything that can be referred to: a character, a group, a
place, an object. Each has an **entity type**, which declares its fields and
its **kind**:

- `agent`: can hold knowledge (a person, an NPC, a reader);
- `group`: can hold knowledge of its own (a guild, a council, a party);
- `other`: can be talked about but holds nothing (a place, an object).

An entity may carry an **external reference** (`{ system, id }`), the id
another system gives it, so a game or an app can register and find its own
characters by their ids (`entity.ensureBatch`, `entity.resolve`).

## Claims

A **claim** is a statement: a **subject** (an entity), a **predicate**, an
**object** (an entity, a value, or nothing), optional **qualifiers**, a
**polarity** (true, or false for "not") and optionally the record it is
**about**.

**Predicates** are declared per project: whether their object is an entity, a
value or nothing, and their **cardinality**. A predicate of cardinality `one`
is functional: a subject has one value for it at a time (where someone lives),
so two different values in the same slot are a contradiction. Predicates
beginning with `umwelt:` are reserved and have built-in meaning, such as
`umwelt:member-of` (group membership), `umwelt:name` and `umwelt:alias`
(names over time) and `umwelt:holds` (what one holder thinks another knows).

Every claim has a **claim key**, a stable hash of what it says that ignores
polarity, so "Bob is in the castle" and "Bob is not in the castle" share a
key. Functional claims also have a **slot key** (subject and predicate). See
[claim-key.md](../spec/claim-key.md).

## Records

Everything Umwelt knows is a **record**, written once and never edited. There
are four kinds:

- **Fact**: canon, what is true in the world. A Fact is not anyone's
  knowledge: knowing that Bob is the thief requires a Holding.
- **Holding**: a claim held by a **holder** (an agent or a group), with:
  - an **attitude**: `knows`, `believes`, `suspects` or `remembers`;
  - an **acquisition**, how the holder came to hold it: `witnessed` (at an
    event), `told-by` (another holder, possibly repeating one of their own
    Holdings, honestly or deceptively), `evidence`, `inferred` (from other
    records) or `authored`;
  - a **confidence** between 0 and 1;
  - an **accessibility**: `available`, or `suppressed` / `inaccessible` for
    knowledge the holder has but cannot reach (a repressed memory, something
    forgotten for now). Such Holdings are **withheld**: reads leave them out
    unless asked for them;
  - optional free **text**, and optional **secrecy**: who the holder may
    share it with.
- **Awareness**: a holder knows that an entity exists, and nothing else about
  it.
- **Disposition**: how a holder stands toward a target along a named
  **dimension** (trust, fear, affection), with a value. Dispositions are
  directional: what Ana feels toward Bob says nothing about what Bob feels
  toward Ana.

A belief that contradicts canon is still a valid Holding. Umwelt keeps it and
reports the contradiction (`contradictions`); it never corrects it.

## Events and transitions

Records are written in **events**: something that happened (a scene, a day, a
conversation), with a type, a title, a position, optional participants,
location and evidence. Being a participant or being present grants nothing:
only records grant knowledge.

A record changes state through a **transition**, never by being edited or
deleted. A transition makes a record inactive from its position on, with a
status that names why: `retracted`, `revoked`, `forgotten`, `abandoned`, or
`superseded` (replaced by another record it names). The engine treats them
alike; the status is the client's account of what happened. The history stays
readable: before the transition, the record is still active.

## Positions and versions

Every read happens at a **position** and a **version**:

- the **position** (`tick`) is where in the story or game the read stands. A
  record is visible from its position on, so "what did Ana know in chapter 6"
  reads tick 6;
- the **version** (`asOfVersion`) is how much of the ledger is considered.
  Every commit gets a new version, so a read can ignore what was written
  later, even about earlier positions: "what did we know about chapter 6
  before last week's revision".

Both default to the latest. A read never sees anything later on either.

A project can declare **additional time axes** (`axis.define`), such as
reading order next to story order. An event may sit at a position on each
axis; a read picks its axis (`at: { axis: "reading", tick: 12 }`). A record
whose event has no position on that axis is visible to nobody there.

## Perspectives

Most reads are made **for a holder**: `holds` (does Ana hold this claim?),
`holdings` (what does she hold?), `perspective` (everything she holds, is aware
of and feels), `contradictions`, `entity.asKnownBy` (what she knows about
someone, under the names she was told). The narrator's reads, such as `canon`
and `entity.at`, see Facts instead. A holder's perspective never contains a
Fact, and never contains another holder's records: group knowledge is the
group's own Holdings, not the union of its members'.

## Writing

A **commit** writes one event with its records and transitions, atomically: all
of it or nothing. Every commit carries an **attempt id**; sending the same
commit again with the same attempt id returns the original outcome instead of
writing twice, so a client can retry safely. An optional **expected version**
makes the commit fail if the timeline moved since the client last read it.

Higher-level writes build commits for common cases: `transmit` (one holder
tells another), `witness` (everyone at a scene sees the same claims),
`holding.adjustBatch`, `transition.batch` and `disposition.setBatch` (changes
to many records at once, such as a night of forgetting).

A **proposal** holds a commit for review instead of applying it: each
operation is accepted or rejected with a rationale, and only the accepted ones
are applied.

## Timelines and drafts

**Main** is the confirmed history. A **draft** is a separate timeline that
branches from main, or from another draft, and sees its parent as it was when
the draft was opened. Work written to a draft is invisible elsewhere until the
draft is **promoted** into its parent; a draft can also be **rebased** onto its
parent's new state, or **discarded** and **purged**. A day of play or a
generated scene can wait in a draft until someone approves it. See
[timelines.md](../spec/timelines.md).

## Search

`search` finds documents (Facts, Holdings with text, events) by their words,
by their meaning, or both, always within what the read can see. Semantic
search uses vectors made by the client's own embedding model, stored in
**embedding spaces** (one per model). Umwelt never runs a model. See
[search.md](../spec/search.md).

## Context admission

`context.admit` decides which candidate records and events may enter a
context, such as a prompt for a model voicing a character: only what the
character holds at that position, within a budget, with a reason for every
refusal (`future`, `not-held`, `withheld`, `over-budget`...).
