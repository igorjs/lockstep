<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-inventory

Items, containers, equipment, affixes and spoilage for [Lockstep](https://github.com/igorjs/lockstep),
all in integers.

- `Catalogue`: every kind of item, equipment slot and affix, and the spoilage bands, read from JSON
  and checked against a `lockstep-attributes` registry.
- `Inventory`: items as entities in their own store, containers and wearers keyed by the
  simulation's own handles. `put`, `take`, `move_between` (all or nothing), `split`, `consume`,
  `destroy` and `find_by_tag`.
- Equipment: `equip` and `unequip` add and remove the kind's and the affixes' modifiers on the
  wearer's attributes. An affix that binds keeps the item on until it is removed.
- Spoilage: `spoil` adds up exact exposure, game minutes times a percent for the temperature, so
  the same minutes spoil the same at any step rate.

## Example

```rust
use lockstep_attributes::Registry;
use lockstep_core::math::Fixed32;
use lockstep_core::StableVector;
use lockstep_inventory::{Catalogue, Container, Inventory};

let registry = Registry::from_json(r#"{ "attributes": [] }"#).unwrap();
let catalogue = Catalogue::from_json(r#"{
  "spoilage": [ { "below": 5, "percent": 25 }, { "percent": 100 } ],
  "kinds": [
    { "name": "ration", "tags": ["food"], "weight": "0.5", "stack": 6, "spoils_after_minutes": 1440 }
  ]
}"#, &registry).unwrap();
let ration = catalogue.kind_id("ration").unwrap();

let fridge = StableVector::new().insert(());
let mut inventory = Inventory::new();
inventory.add_container(fridge, Container::new(4, None, 2)).unwrap();
let stack = inventory.create(&catalogue, ration, 3);
inventory.put(stack, fridge, &catalogue).unwrap();

// A day in the fridge at a quarter of the rate: three quarters fresh.
let mut events = Vec::new();
inventory.spoil(Fixed32::from_int(1_440), 20, &catalogue, &mut events);
assert_eq!(inventory.freshness(stack, &catalogue), Some(Fixed32::from_ratio(3, 4)));
assert_eq!(inventory.find_by_tag(fridge, "food", &catalogue), vec![stack]);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-inventory.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
