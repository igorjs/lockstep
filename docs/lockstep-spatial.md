# lockstep-spatial

A deterministic spatial toolkit: cells as integer indices, a topology trait, and dense maps with
elevation, and a packed occupancy set for bodies. Collision is not detection: it is occupancy.
The first caller to claim a cell gets it, so a simulation resolves contested cells in handle order by
applying its moves in handle order. Pathfinding, flow fields and line of sight follow in the next part of
milestone M3.

## Cells and topologies

`Cell(u32)` is `y * width + x`. A topology decides how cells connect, and everything else takes it as a
type parameter.

| Topology | Neighbours | Distance (tenths) |
| --- | --- | --- |
| `Square8` | 8, clockwise from north: N, NE, E, SE, S, SW, W, NW | `10 * max(dx, dy) + 4 * min(dx, dy)`, so 10 straight and 14 diagonal |
| `Square4` | 4: N, E, S, W | Manhattan, 10 per step |
| `Hex` (feature `hex`) | 6, clockwise from north-east: NE, E, SE, SW, W, NW | cube distance, 10 per step; odd rows shifted right |

North is a smaller y. Neighbour order is fixed, because determinism depends on it. `line` returns the
cells from one cell to another, both ends included, using integer arithmetic only. Square lines are
symmetric (A to B passes the same cells as B to A), and hex lines never leave the map.

## GridMap

`GridMap<T>` holds, per cell: passable, cost, elevation, and wall height. `can_step(from, to)` needs a
passable target, a climb within `step_limit` (a drop is always allowed), and no corner cutting on a
diagonal. `step_cost` is the topology cost times the terrain cost of the cell stepped onto.
`set_passable(cell, false)` makes a full wall; `set_low_wall(cell, height)` makes a wall that blocks sight
only when it reaches the line of sight (used by the next part). Every change marks a 32 by 32 chunk dirty,
and `take_dirty_chunks()` returns and clears them in ascending order, for hosts and caches. The dirty
marks are not state: saves, hashes and equality ignore them. Loading a map checks it as `new` does,
and a loaded map reports every chunk as changed.

## Occupancy

A packed sparse set: cell to slot, a dense list of bodies, and a sorted handle index. `at` is constant
time, `cell_of` is logarithmic, and moving a placed body is constant time plus its footprint; placing a
new body and `vacate` shift the sorted index, so they are linear in the number of bodies. It saves as
the bodies sorted by handle, so equal occupancies save, compare and hash the same whatever their history,
and a load checks every cell is inside the grid and held once. A body may hold several cells (a
footprint): `move_footprint` is atomic, so a blocked move changes nothing. `within(map, centre,
radius, out)` returns occupied cells inside a radius (in tenths), sorted by cell, whatever order the
bodies were placed and moved in.

## Tests

- `tests/behaviour`: topologies, the map, occupancy against a plain model over random histories.
- `tests/decisions`: the circle measure for topologies, sorted queries, atomic footprints.
- Every test also runs under WebAssembly, with and without the `hex` feature.
- The `hex` tests run with `cargo test -p lockstep-spatial --features hex`, which `just test` does.
