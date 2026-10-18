<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-tables

Data tables for [Lockstep](https://github.com/igorjs/lockstep): named rows read from JSON, checked
as a whole, and replaced while a session runs.

- `Table<Row>`: rows by name in a stable order, built only by `from_json`, `swap` or decoding (all
  run the same checks), with a `digest` that names its content.
- `TableSwap`: an intent carrying the new table's JSON. The simulation applies it with
  `Table::swap`, which keeps the old table unless the new one passes every check. Because the
  swap is an intent, a recording keeps it and a replay swaps at the same step.

## Example

```rust
use lockstep_tables::{Swapped, Table, TableRow};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Price {
    minor: i64,
}

impl TableRow for Price {
    fn check(&self) -> Result<(), String> {
        if self.minor < 0 {
            Err("a price below zero".to_string())
        } else {
            Ok(())
        }
    }
}

let mut prices = Table::<Price>::from_json(r#"{"rows":[{"name":"tea","minor":300}]}"#).unwrap();
let next = r#"{"rows":[{"name":"tea","minor":350}]}"#;
assert!(matches!(prices.swap(next), Ok(Swapped::Replaced { .. })));
assert_eq!(prices.get("tea"), Some(&Price { minor: 350 }));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-tables.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
