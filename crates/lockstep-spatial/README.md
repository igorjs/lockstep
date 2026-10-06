<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-spatial

Deterministic grids for [Lockstep](https://github.com/igorjs/lockstep): integer arithmetic only, so a
path is the same cell by cell on every platform.

- Topologies: `Square8`, `Square4`, and `Hex` (feature `hex`), with a fixed neighbour order.
- `GridMap`: passable, terrain cost, elevation and low walls per cell, with dirty chunks for hosts.
- `Occupancy`: which body holds which cell. Collision is occupancy: the first caller gets the cell.
- `Pathfinder`: A* with reused buffers and a `(cost, cell)` tie break.
- `find_paths`: a batch of paths, on a thread pool with the `parallel` feature, with the same answers
  as the serial batch.
- `FlowField`: one search toward many targets, for hordes.
- `line_of_sight`: from a viewer's eye to a target's feet, over elevation and low walls.

## Example

```rust
use lockstep_spatial::{GridMap, Occupancy, PathOptions, PathResult, Pathfinder, Square8};

let mut map: GridMap<Square8> = GridMap::new(16, 16);
for y in 2..14 {
    map.set_passable(map.index(8, y), false); // a wall with gaps at both ends
}
let occupancy = Occupancy::new(&map);
let mut pathfinder = Pathfinder::new(&map);
let mut path = Vec::new();
let result = pathfinder.find(
    &map,
    &occupancy,
    map.index(2, 8),
    map.index(14, 8),
    PathOptions::default(),
    &mut path,
);
assert!(matches!(result, PathResult::Found { .. }));
assert_eq!(path.last(), Some(&map.index(14, 8)));
assert!(path.iter().all(|cell| map.is_passable(*cell)));
```

## Features

| Feature | Adds |
|---|---|
| `hex` | The hexagonal topology. |
| `parallel` | `find_paths` on a `rayon` thread pool. WebAssembly always runs the batch serially. |

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-spatial.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
