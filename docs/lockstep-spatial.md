# lockstep-spatial

A deterministic spatial toolkit: cells as integer indices, a topology trait, dense maps with elevation,
a packed occupancy set for bodies, an A* pathfinder, flow fields for hordes, and integer line of sight.
Collision is not detection: it is occupancy. The first caller to claim a cell gets it, so a simulation resolves contested cells in handle order by
applying its moves in handle order.

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

## Pathfinder

`Pathfinder` is A* with buffers reused across searches and reset in constant time by a generation
stamp. `find(map, occupancy, from, to, options, out)` fills `out` with the cells to walk (excluding the
start, including the goal) and returns `Found { cost }` (in tenths), `Unreachable`, or `Aborted` when
`maximum_expansions` runs out. It respects walls, terrain cost, the climb limit, and corner cutting, and
`treat_occupants_as_walls` makes held cells walls except the destination. The open set is ordered by
`(f_cost, cell)` and neighbours come in a fixed order, so equal-cost paths are identical on every
platform. `last_expansions()` reports the work of the last search.

## Flow fields

`FlowField::build(map, targets, maximum_distance)` runs one search outward from the targets (Dijkstra,
because costs differ) and stores, for each cell, the cost to the nearest target and the next cell to step
to. `distance(cell)` and `step_from(cell)` are table lookups, so a hundred bodies chasing one target cost
one build. Every next step is strictly closer to a target, and walls are never part of the field.

## Line of sight

`line_of_sight(map, from, to, eye_height, line)` draws the topology's line and compares the top of each
cell between (its elevation plus its wall height) with the straight line from the viewer's eye down or up
to the floor of the target cell. A cell that reaches the line blocks the view. A full wall is 255 steps
tall; a low wall blocks only when it reaches the line, so a terrace sees over a garden wall at its foot
and the ground does not see up over it. `line_of_sight_symmetric` needs both directions.

## Benchmark

`just bench` runs `benches/spatial.rs` and compares with `benches/baseline.txt`, warning when anything
is more than 10 percent slower. `just bench-baseline` writes a new baseline after an intended change.
It measures 500 paths on a 512 by 512 map with 20 percent walls, `within` at radius 24 for 1,000 agents,
and one flow field. See decision 0005 for the paths measurement against the reference's target.

## Tests

- `tests/behaviour`: topologies, the map, occupancy against a plain model over random histories, the pathfinder (including a cross-check against the flow field), flow fields, and line of sight.
- `tests/decisions`: the circle measure for topologies, sorted queries, atomic footprints, elevation and sight, flow fields, and the path tie break with its committed hash.
- `tests/spikes`: forcing the generation counter to wrap.
- Every test also runs under WebAssembly, with and without the `hex` feature.
- The `hex` tests run with `cargo test -p lockstep-spatial --features hex`, which `just test` does.
