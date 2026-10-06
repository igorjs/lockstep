<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-inventory

Items, containers, equipment, affixes and spoilage for [Lockstep](https://github.com/igorjs/lockstep),
all in integers. This first part is the catalogue; the inventory itself follows.

- `Catalogue`: every kind of item, equipment slot and affix, and the spoilage bands, read from JSON
  and checked against a `lockstep-attributes` registry.

## Example

```rust
use lockstep_attributes::Registry;
use lockstep_inventory::Catalogue;

let registry = Registry::from_json(r#"{ "attributes": [] }"#).unwrap();
let catalogue = Catalogue::from_json(r#"{
  "spoilage": [ { "below": 5, "percent": 25 }, { "percent": 100 } ],
  "kinds": [
    { "name": "ration", "tags": ["food"], "weight": "0.5", "stack": 6, "spoils_after_minutes": 1440 }
  ]
}"#, &registry).unwrap();
let ration = catalogue.kind(catalogue.kind_id("ration").unwrap());
assert_eq!(ration.stack, 6);
assert!(ration.has_tag("food"));
assert_eq!(catalogue.spoilage_percent(2), 25);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-inventory.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
