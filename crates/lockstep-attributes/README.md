<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-attributes

Stats as data for [Lockstep](https://github.com/igorjs/lockstep). Every stat is the same machine:
health, sanity, hunger, a rover's battery, a credit limit.

- `Registry`: every attribute an entity can have, in a stable order, read from JSON and checked.
  Numbers are whole numbers or decimal strings, never JSON fractions, so they read the same
  everywhere.
- `Attributes`: one entity's values. A change is clamped, and each threshold fires once per crossing.
- `Modifier`: changes a maximum, applied Add, then Multiply, then the last Override, whatever order
  they were added in. Each attribute picks whether a new maximum clamps, scales or ratchets its value.
- `Curve`: how a derived attribute turns its inputs into a value (linear, piecewise, threshold,
  product, sum, difference), saturating instead of wrapping.

- `Effects`: timed effects that hold a modifier or drain per game minute, with four stacking rules
  (independent, refresh, replace, and a daily budget). One minute in one tick or in 1,800 ticks gives
  the same result.

## Example

```rust
use lockstep_attributes::{AttributeEvent, Attributes, Modifier, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::StableVector;

let registry = Registry::from_json(r#"{
  "attributes": [
    { "name": "health", "minimum": 0, "maximum": 100, "starting": 50,
      "on_maximum_change": "scale_current",
      "thresholds": [ { "at": 20, "name": "wounded" } ] },
    { "name": "luck", "minimum": 0, "maximum": 20, "starting": 3 },
    { "name": "critical_chance", "minimum": 0, "maximum": 100, "starting": 0,
      "derived": { "inputs": ["luck"], "curve": { "linear": { "per_point": "1.5", "offset": 5 } } } }
  ]
}"#).expect("a valid registry");
let (health, luck, critical) = (
    registry.id("health").unwrap(),
    registry.id("luck").unwrap(),
    registry.id("critical_chance").unwrap(),
);

let who = StableVector::new().insert(());
let mut attributes = Attributes::from_registry(&registry);
let mut events = Vec::new();

// 50 of 100 with +50 percent becomes 75 of 150.
attributes.add_modifier(who, health, Modifier::Multiply(Fixed32::from_ratio(3, 2)), &registry, &mut events);
assert_eq!(attributes.get(health).current(), Fixed32::from_int(75));

// Falling below 20 fires "wounded" once.
attributes.apply(who, health, Fixed32::from_int(-60), &registry, &mut events);
assert!(matches!(&events[..], [AttributeEvent::Crossed { upward: false, .. }]));

// Critical chance follows Luck: 1.5 per point, plus 5.
attributes.apply(who, luck, Fixed32::from_int(1), &registry, &mut events);
assert_eq!(attributes.get(critical).current(), Fixed32::from_int(11));
```
See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-attributes.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
