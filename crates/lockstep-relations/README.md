<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-relations

Standings for [Lockstep](https://github.com/igorjs/lockstep): how one entity stands toward
another, or toward a group, in kinds such as trust and reputation.

- `Kinds`: relations (bounds, a starting value, a rest it drifts back to, a decay a game day,
  named thresholds) and groups, read from JSON and checked.
- `Relations`: directional standings. `change` moves one within bounds and emits the thresholds
  it crosses; `tick` lets game time pass, drifting each standing toward rest exactly, whatever the
  step rate; `toward` adds the standing toward each group an entity belongs to; `forget` drops an
  entity's standings.

## Example

```rust
use lockstep_core::math::Fixed32;
use lockstep_core::StableVector;
use lockstep_relations::{Kinds, Relations, Target};

let kinds = Kinds::from_json(r#"{
  "relations": [ { "name": "trust", "minimum": -100, "maximum": 100, "starting": 0,
                   "decay_per_day": 10, "thresholds": [ { "at": 50, "name": "trusted" } ] } ]
}"#).unwrap();
let trust = kinds.relation_id("trust").unwrap();
let mut store = StableVector::new();
let (driver, dispatcher) = (store.insert(()), store.insert(()));
let mut relations = Relations::new();
let mut events = Vec::new();
relations.change(&kinds, trust, driver, Target::Entity(dispatcher), Fixed32::from_int(55), &mut events);
// A game day later it has drifted 10 toward its rest of 0.
relations.tick(&kinds, Fixed32::from_int(1_440), &mut events);
assert_eq!(relations.get(&kinds, trust, driver, Target::Entity(dispatcher)), Fixed32::from_int(45));
// Directional: the dispatcher's trust in the driver is untouched.
assert_eq!(relations.get(&kinds, trust, dispatcher, Target::Entity(driver)), Fixed32::ZERO);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-relations.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
