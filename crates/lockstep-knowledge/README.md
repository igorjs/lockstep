<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-knowledge

What a knower has found out, for [Lockstep](https://github.com/igorjs/lockstep): facts learned from
fragments, questions that open after enough of them, and rules over predicates that fire once.

- `Catalogue`: facts, questions and rules (revelations, secrets, achievements) read from JSON.
  Predicates are data (`known`, `fragments`, `opened`, `fired`, `happened`, `all`, `any`, `not`),
  so a catalogue can be saved, compared and hashed.
- `enrol` makes an entity a knower. `receive`: a fragment of a fact from a source; fragments from
  distinct sources make a fact known and open questions, each once. `notice`: counts the events
  that mention each knower, by kind, for `happened`.
- `evaluate`: fires every rule whose predicate holds, once per knower.

## Example

```rust
use lockstep_core::{Column, StableVector};
use lockstep_knowledge::{enrol, evaluate, receive, Catalogue, Fragment, KnowledgeEvent};

let catalogue = Catalogue::from_json(r#"{
  "facts": [ { "name": "round_amounts", "fragments": 2 } ],
  "rules": [ { "name": "suspicion", "kind": "revelation", "when": { "known": "round_amounts" } } ]
}"#, &[]).unwrap();
let fact = catalogue.fact_id("round_amounts").unwrap();
let auditor = StableVector::new().insert(());
let mut knowledge = Column::new();
enrol(&mut knowledge, auditor);
let mut events = Vec::new();
// Two sources make it known; the rule fires once.
receive(&mut knowledge, auditor, Fragment { fact, source: 1 }, &catalogue, &mut events);
receive(&mut knowledge, auditor, Fragment { fact, source: 2 }, &catalogue, &mut events);
evaluate(&mut knowledge, &catalogue, &mut events);
evaluate(&mut knowledge, &catalogue, &mut events);
let fired = events.iter().filter(|event| matches!(event, KnowledgeEvent::Fired { .. })).count();
assert_eq!(fired, 1);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-knowledge.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
