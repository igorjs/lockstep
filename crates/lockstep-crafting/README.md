<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-crafting

Crafting for [Lockstep](https://github.com/igorjs/lockstep): recipes that consume their inputs all
or nothing, take game time at a station, and roll their outcome from a table.

- `Recipes`: station kinds and recipes (inputs, minutes, a weighted outcome table whose rows can
  be failure branches) read from JSON and checked against an inventory catalogue.
- `Crafting`: stations and their jobs. `start` takes every input or none, spoiled units not
  counting; `tick` lets game time pass, draws each finished job's outcome from the `"crafting"`
  stream by weight, and puts the outputs into the station's container, or leaves them loose.
- `roll`: one draw below the total weight, so each row's share converges on its weight.

## Example

```rust
use lockstep_core::Streams;
use lockstep_crafting::roll;

// 85, 10 and 5: a hundred thousand draws land within half a percent of each.
let mut streams = Streams::new(7);
let mut counts = [0u32; 3];
for _ in 0..100_000 {
    counts[roll([85u32, 10, 5].iter().copied(), &mut streams)] += 1;
}
assert!(counts[0].abs_diff(85_000) <= 500);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-crafting.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
