# What Umwelt is for

Umwelt tracks **who knows what, when, and how you know they know it**. That question shows up in four places that normally solve it four incompatible ways. Umwelt solves it once, and none of these paths requires an AI.

The primitives are the same everywhere: an Entity is anything that can be referred to; a record is one thing established at one position; a query answers *through a position* and *for a perspective*; a transition changes state without deleting history. Examples below use `umwelt call <file> <op> <json>`; every operation is also a typed method of the TypeScript client (`@kallamo/umwelt`) and the .NET binding (`Kallamo.Umwelt`), and is available to any language over the protocol or the C ABI.

---

## 1. Tabletop RPG and long campaigns

**Problem.** The GM knows everything. Each player knows a different subset. Somebody rolls Insight and learns one fact. Three sessions later nobody remembers who was in the room, and the stranger the party met in session 2 is now the king under another name.

**How it maps.**

- Each character is an Entity; the party is a Group whose membership is a Fact with a position.
- A session is an Event. Being present grants nobody knowledge: `witness` records, per character, what each one actually saw and how clearly.
- A successful roll writes one Holding for one character. A lie the party believes is a Holding that contradicts canon. It stays held, and `contradictions` reports it.
- Player recap: `perspective` for that character. GM screen: `compare` between two players, or `groupCoverage` to see who in the party still does not know.
- Names and titles change over time (`entity.at`), and each character knows the names they were told (`entity.asKnownBy`).
- "She thinks he knows" is a Holding about another holder's knowledge; `beliefsAbout` compares it with what he really holds.

```bash
umwelt call campaign.umw perspective '{"projectId":"west-marches","holder":"thaliel"}'
umwelt call campaign.umw compare '{"projectId":"west-marches","a":"thaliel","b":"bram"}'
```

A character sheet is a query, not a document. Ask at session 4's position and you get what they knew then.

---

## 2. Novel and series continuity

**Problem.** Chapter 31 contradicts chapter 6. A character reacts to something they have not been told yet. A revision changes a fact and you cannot find everything downstream that assumed it.

**How it maps.**

- Narrative position is chapter or scene order. **Future records are excluded from earlier reads**, so "what did she know in chapter 6" is a query, not an act of memory.
- Revising canon is a `superseded` transition: the old fact stays readable at every earlier position.
- `check.utterance` checks a line of dialogue (as claims extracted by you or a model) against what the speaker can know: `holds`, `holds-opposite`, `future` (an anachronism), `withheld`, `unknown`, plus whether canon agrees.
- `trace` answers "how does she know this": every link of who told whom.
- A chapter in progress lives in a draft timeline and becomes canon when it is promoted.
- Story order and reading order are two clocks. Declare a `reading` axis and place each scene on both: a flashback happens at story tick 3 and is read in chapter 12. "What does the reader know in chapter 5" reads `at: { axis: "reading", tick: 5 }`; "what does the father know at that point of the story" reads the story tick. `compare` with `atB` sets the reader against a character, each on their own clock.

```bash
umwelt call novel.umw axis.define '{"projectId":"saga","axis":{"id":"reading","label":"Reading order"}}'
umwelt call novel.umw holdings '{"projectId":"saga","holder":"reader","at":{"axis":"reading","tick":5}}'
```

---

## 3. Game mods and systems without AI

**Problem.** Dialogue gating is a pile of boolean flags. `knows_about_betrayal = true` says nothing about when, from whom, or whether it survived a later revelation. Save files diverge and nobody can explain why an NPC said the wrong line.

**How it maps.**

- The mod loads the engine in-process (the .NET binding, or the C ABI from any language), or spawns `umwelt serve save.umw`. No SQL.
- `entity.ensureBatch` registers the game's characters by their own ids when a save loads.
- The day being played is a draft: promote it when the player sleeps, discard it with `purge` when they quit without saving.
- A gate is a query: does this NPC hold this knowledge at this position? Gossip is `transmit`; a scene is `witness`; the night's forgetting and trust changes are `holding.adjustBatch`, `transition.batch` and `disposition.setBatch`. The game decides the rules; the engine records them.
- Secrets carry who may know them. Telling one to the wrong person is never blocked: it returns a `SECRET_BREACH` warning and shows up in `breaches`.
- Hooks fire more than once, after a reload or a rewind. Pass `attemptId` and a replayed commit applies exactly once.

```json
{"v":1,"id":"1","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"game-mod/1"}}
{"v":1,"id":"2","op":"holds","payload":{"projectId":"save","holder":"npc-guard","claimKey":"<claim key>","at":{"tick":420}}}
```

The change feed (`changes`) is append-only, so a mod can poll it to react to what another system just recorded.

---

## 4. AI and prompt construction

**Problem.** The model is told everything and then asked to pretend it does not know. It leaks. Retrieval returns whatever embeds close to the query, including the twist.

**How it maps.**

- `context.admit` admits candidates for one perspective within a budget and returns a **reason for every refusal**: `future`, `inactive`, `not-held`, `withheld`, `excluded-classification`, `duplicate`, `over-budget`. Nothing is dropped silently, so when the model gets something wrong you can see whether the context was at fault.
- A character-voiced generation uses that character's perspective; a narrator has none and sees everything visible. The distinction is enforced, not prompted.
- Model output is a *proposal*, not canon: review per operation, apply what you accept, keep the rationale of what you reject. If the story moved on meanwhile, `proposal.rebase` says which operations still fit.
- A generated scene can also be written to a draft and promoted after review.
- Finding what is relevant is a search by meaning: the client's embedding model turns texts into vectors, Umwelt stores them and searches them **within the same perspective**. "Who took the necklace?" finds "someone stole the duchess's jewel" in what this character holds, never a twist they have not learned. See [../spec/search.md](../spec/search.md).

Umwelt treats generated content as something to review rather than something to trust, using the same machinery a human editor would use.

---

## What Umwelt deliberately does not do

- **It does not generate or read free text.** No model is called anywhere in the engine; claims to check and vectors to search come from the client.
- **It does not resolve contradictions.** A belief against canon is reported, never corrected.
- **It does not infer.** Presence, awareness and group membership never become knowledge. Every grant is an explicit record.
- **It does not apply policy.** Decay, trust and secrecy rules belong to the client; the engine records and reports.
- **It does not delete confirmed history.** Only drafts that were never promoted can be purged.
- **It is not a database you query directly.** SQLite is an implementation detail behind the engine.

If you want something that guesses what a character probably knows, this is the wrong tool. Umwelt is for when you need to be able to prove it.
