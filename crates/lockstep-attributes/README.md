<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-attributes

Stats as data for [Lockstep](https://github.com/igorjs/lockstep). Every stat is the same machine:
health, sanity, hunger, a rover's battery, a credit limit.

- `Registry`: every attribute an entity can have, in a stable order, read from JSON and checked.
  Numbers are whole numbers or decimal strings, never JSON fractions, so they read the same
  everywhere.
- `Curve`: how a derived attribute turns its inputs into a value (linear, piecewise, threshold,
  product, sum, difference), saturating instead of wrapping.

Modifiers (Add, then Multiply, then Override), thresholds that fire once per crossing, and timed
effects are in progress in milestone M4.

## Example

```rust
use lockstep_attributes::{Curve, Registry};
use lockstep_core::math::Fixed32;

let registry = Registry::from_json(r#"{
  "attributes": [
    { "name": "luck", "minimum": 0, "maximum": 20, "starting": 3 },
    { "name": "critical_chance", "minimum": 0, "maximum": 100, "starting": 0,
      "derived": { "inputs": ["luck"], "curve": { "linear": { "per_point": "1.5", "offset": 5 } } } }
  ]
}"#).expect("a valid registry");

let critical = registry.definition(registry.id("critical_chance").unwrap());
let curve: &Curve = &critical.derived.as_ref().unwrap().curve;
assert_eq!(curve.evaluate(&[Fixed32::from_int(4)]), Fixed32::from_int(11));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-attributes.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
