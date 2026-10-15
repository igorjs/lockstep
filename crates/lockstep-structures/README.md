<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-structures

Durable multi-cell structures for [Lockstep](https://github.com/igorjs/lockstep): a structure is
an entity whose cells are walls on a map while it stands. It is worn down, mended, and gone when
its durability runs out.

- `Structures::build` raises a structure over open cells, all or nothing.
- `damage`, `repair` and `dismantle` change it; a collapse or a dismantling opens the cells.
- Every change goes through the map's own setters, so the chunks that hold the cells are marked
  dirty for hosts and caches.

## Example

```rust
use lockstep_core::StableVector;
use lockstep_spatial::{GridMap, Square4};
use lockstep_structures::Structures;

let mut map: GridMap<Square4> = GridMap::new(8, 8);
let mut owners = StableVector::new();
let barricade = owners.insert("barricade");
let mut structures = Structures::new();
let mut events = Vec::new();
let cells = [map.index(3, 2), map.index(3, 3), map.index(3, 4)];
structures.build(&mut map, barricade, &cells, 10, &mut events).unwrap();
assert!(!map.is_passable(cells[1]));
structures.damage(&mut map, barricade, 10, &mut events).unwrap();
assert!(map.is_passable(cells[1]));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-structures.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
