<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-knowledge

What a knower has found out: facts learned from fragments, questions that open after enough
evidence, and revelations, secrets and achievements as rules over predicates that fire once.

## The catalogue

`Catalogue::from_json(text, event_kinds)` reads facts, questions and rules, and checks them.
`event_kinds` names the simulation's event kinds (the `Indexable::kind` of each event) for
`happened`.

```json
{
  "facts": [
    { "name": "round_amounts", "fragments": 2 },
    { "name": "shared_address", "fragments": 1 }
  ],
  "questions": [
    { "name": "who_is_behind_it", "facts": ["round_amounts", "shared_address"], "fragments": 2 }
  ],
  "rules": [
    { "name": "suspicion", "kind": "revelation", "when": { "known": "round_amounts" } },
    { "name": "ring", "kind": "secret",
      "when": { "all": [ { "fired": "suspicion" }, { "known": "shared_address" } ] } },
    { "name": "busy", "kind": "achievement",
      "when": { "happened": { "event": "transfer", "at_least": 3 } } }
  ]
}
```

- A fact is known after `fragments` fragments from distinct sources.
- A question opens once its facts together have `fragments` fragments, known or not.
- A rule has a kind (`revelation`, `secret` or `achievement`, the simulation decides what each
  means) and a predicate: `known`, `fragments` (at least so many of a fact), `opened`, `fired`
  (only a rule listed earlier), `happened` (at least so many events of a kind mentioning the
  knower), `all`, `any` and `not`. `all` of nothing holds; `any` of nothing does not.
- Refused with a reason: repeated names, a fact or question needing no fragments, a question with
  no facts or the same fact twice, an unknown fact, question, rule or event, and a rule naming a
  later rule (`RuleOrder`).

Predicates are data, so a catalogue can be saved, compared and hashed (decision 0023).

## Knowledge

Each knower has a `Knowledge` in a `Column`: its fragments, the events it has noticed by kind, and
the facts, questions and rules already learned, opened and fired. All of it is saved, so nothing
fires twice after a load.

- `enrol(knowledge, knower)` makes an entity a knower, with nothing known. Only enrolled knowers
  receive fragments, notice events and have rules evaluated, so a stale handle never starts
  knowledge of its own in another entity's slot.
- `receive(knowledge, knower, fragment, catalogue, events)` adds a fragment of a fact from a
  source (a document, a witness, an instrument, as a number). The same source for the same fact
  again changes nothing. It emits `Learned` when the fact reaches its fragments and `Opened` for
  each question that reaches its own, each once. A fact the catalogue lacks is ignored.
- `notice(knowledge, events)` reads a step's events as they happen and counts each one, by its
  kind, for every enrolled knower it mentions. `happened` reads those counts, so they are saved
  with the knowledge and compacting or dropping the timeline changes nothing.
- `evaluate(knowledge, catalogue, events)` fires each rule whose predicate holds, once per enrolled
  knower, in handle order and then rule order (`Fired`, with the rule's kind). A rule naming an
  earlier rule sees it fired in the same pass. A rule that has fired stays fired even when its
  predicate stops holding.
- Saved knowledge names facts, questions and rules by their place in the catalogue. Store
  `Catalogue::hash` with a save and check it at load: a catalogue edited since needs the save
  converted first (conversions wait for saves, deferral D4).

## Tests

- `tests/behaviour/catalogue.rs`: every predicate read from JSON, and each refusal.
- `tests/behaviour/knowledge.rs`: a question opening after two fragments across its facts, a rule
  firing after an earlier one in the same pass, `not` and `any`, `happened` counting only the
  events that mention the knower and surviving a save, and a knower not enrolled, a stale handle
  and an unknown fact all ignored.
- `tests/decisions`: rules fire once, through a hundred evaluations and a save and load; a source
  counts once; and the test catalogue hashes to `tests/fixtures/predicates.hash` natively and
  under WebAssembly (decision 0023).
