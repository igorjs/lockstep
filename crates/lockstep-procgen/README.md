<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-procgen

Procedural generation for [Lockstep](https://github.com/igorjs/lockstep): a map assembled from
chunk templates drawn from weighted pools, with reserved chunks, and a seed for every chunk that
never collides with another.

- `Templates`: square chunk templates and pools of them, read from JSON and checked.
- `Templates::roll`: a layout of chunks, each drawn from a pool by weight from its own stream, or
  fixed by a `Reservation`.
- `chunk_seed`: the seed of one chunk, a function of the world seed and its place alone.
- `Layout::apply`: writes the layout onto a `GridMap`, marking the changed chunks dirty.

## Example

```rust
use lockstep_procgen::{chunk_seed, Templates};

let templates = Templates::from_json(
    r##"{ "side": 2,
         "templates": [ { "name": "meadow", "rows": ["..", ".."] },
                        { "name": "rocks", "rows": ["#.", ".#"] } ],
         "pools": [ { "name": "wild", "entries": [
             { "template": "meadow", "weight": 3 }, { "template": "rocks", "weight": 1 } ] } ] }"##,
)
.unwrap();
let wild = templates.pool_id("wild").unwrap();
let layout = templates.roll(7, 4, 4, wild, &[]).unwrap();
assert_eq!(layout, templates.roll(7, 4, 4, wild, &[]).unwrap());
assert_ne!(chunk_seed(7, 0, 1), chunk_seed(7, 1, 0));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-procgen.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
