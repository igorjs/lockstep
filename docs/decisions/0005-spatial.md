<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0005 The spatial crate (decided, with two open items)

Status: decided and implemented for `lockstep-spatial` and its consumers, with two open items (the hexagon figure and
the pathfinding benchmark target). The reference calls this crate `lockstep-grid`;
it was renamed to stay generic.

## Decisions where the reference was silent or ambiguous

- **Directions.** North is a smaller y, and `Square8` lists neighbours clockwise from north. Hex lists them clockwise from north-east.
- **`neighbours` and `line` clear their output first**, so a caller can reuse one buffer.
- **Square lines are symmetric.** A line is always drawn from the smaller cell index and reversed when needed, so the line from A to B passes exactly the cells of the line from B to A. Line of sight needs this, or one body could see another without being seen back.
- **Hex lines never leave the map.** A point exactly between two hexes is a tie. The line breaks it with a tiny nudge one way and then the other, and keeps the hex inside the map. A first version broke ties one fixed way, and on an edge column that picked a hex outside the map, which wrapped into a real cell on the far edge; the self-review of the pull request found it.
- **Saves hold cells, not dirty marks.** The dirty chunk marks are bookkeeping for hosts and caches, so they are left out of saves, hashes and equality: two maps with the same cells are equal whatever history built them. A load checks the same things `new` does (at least one cell, per-cell lists of the right length, passable only where there is no wall) and fails at load, not later. A loaded map reports every chunk as changed, so a host redraws it.
- **`step_cost` equals `distance`** for adjacent cells: 10 straight, 14 diagonal.
- **Climbing.** `step_limit` bounds a climb only. A drop of any height is allowed.
- **No corner cutting.** A diagonal step is refused when either orthogonal neighbour is a wall. The test is topology-neutral: a step that costs more than a straight step is a diagonal.
- **Walls.** The reference says walls block "as elevation + 255" and that a low wall "blocks only if it reaches the interpolated eye line". Both are kept with one extra byte per cell, `wall_height`: `set_passable(cell, false)` is a full wall of 255 on top of the cell's elevation, and `set_low_wall(cell, height)` is a lower wall. Line of sight (next part) compares the top of each cell with the eye line.
- **Occupancy.** A body can hold several cells, with the first as its anchor, so `move_footprint` can move a two by two body atomically. Placing a body that is already placed moves it. `within` scans the window around the centre in row order, so its output is sorted by construction. The first caller to claim a cell gets it: occupancy does not order by handle, so a simulation that resolves contested cells in handle order applies its moves in handle order.
- **Occupancy is state.** It saves as the bodies sorted by handle, so equal occupancies save, compare and hash the same whatever order they were built in, and a load refuses cells outside the grid, a cell held twice, a body saved twice, and empty footprints. The self-review of the pull request found it had no saved form at all.
- **`hex` is a feature.** It is not enabled by default. `just test` and `just check` also run the crate with `--features hex`.

## Pathfinder, flow field and line of sight

- **A path excludes its start and includes its goal.** `from == to` is an empty path with cost zero.
- **Terrain cost multiplies the step cost** (`step_cost(from, to)` is the topology cost times the cost byte of the cell stepped onto, with zero counting as one). The heuristic is the plain topology distance, which stays a lower bound.
- **The flow field is Dijkstra**, because step costs differ. A body on a wall cell is not a starting point: the first version gave wall cells distances, and the tests that check every cell's next step caught it.
- **Line of sight aims at the target's floor.** A line between two eyes is the same line in both directions, so a terrace could not see over a wall that the ground cannot see up over. Aiming from the viewer's eye to the floor of the target makes a wall near a raised eye easier to clear than a wall near a low target, which is what the reference's example describes. An eye height of zero lets flat ground block, so use at least one.
- **Generation counter.** A search that wraps the counter clears the stamps, and a spike forces the wrap and checks the paths do not change.

## Consumers

- **Mars Rovers** (`examples/mars-rovers`) runs on all three topologies. The kata numbers its plateau with y growing northward while the grid's north is a smaller y, so positions are converted at the boundary and the classic answers (1 3 N and 5 1 E) are unchanged. A plateau is stored with a ring of wall cells, so the edge is a wall and every cell a rover stands on has a full neighbour list. Rovers act one instruction per step in handle order: a rover sees the moves of lower handles in the same step, a contested cell goes to the lower handle, and a program added in a step starts in the next one.
- **The capsule** walks A* paths around a wall across its room. This changed its movement on purpose, so its fixture hash changed from `316b312aae0edd6a` to `5f834ccde46abfef` (rule: a move follows the cheapest path around the wall, and a spawn that lands on the wall is redrawn). The Mars Rovers fixture hash is `df95ff68f0f75c69`.

## Open item: the pathfinding benchmark target

The reference gives 50 milliseconds as the target for 500 paths on a 512 by 512 map with 20 percent
walls on the development MacBook. Measured on an Apple M1 Pro with the release profile: 611
milliseconds, about twelve times the target. The search expands about 12,700 cells per path (the
paths average 266 cells) at about 96 nanoseconds per expansion. A target of 50 milliseconds needs
roughly twelve times fewer expansions, which exact A* with the octile heuristic on random walls does
not give. The committed baseline is the honest number. Ways to close the gap, none built: hierarchical
pathfinding, bidirectional search, or a cheaper step (precomputed neighbour tables). This needs the
owner: relax the target, or choose a technique.

## Open item: the hexagon figure in the circle decision

The reference's decision test says a path around a circle of radius 50 deviates from the true
distance by at most 8 percent for `Square8`, at most 3 percent for hexagons, and at least 30 percent
for `Square4`. Measured over every pair of points on that circle, comparing the topology distance
with the true distance between cell centres:

| Topology | Worst error | Reference |
| --- | --- | --- |
| `Square8` | 7.70 percent | at most 8: holds |
| `Square4` | 41.42 percent | at least 30: holds |
| `Hex` | 15.47 percent | at most 3: cannot hold |

The hexagon number cannot be met by any hexagon distance. Counting hexagon steps over-counts by a
factor of 1 / cos 30 degrees, which is 15.47 percent, along the directions halfway between two lattice
axes, and in true geometry that is also more than the octile error. The decision test records the
measured value (between 10 and 16 percent) and does not assert 3. This needs the owner: either the
figure was meant for something else, or hexagons are not more isotropic than `Square8` and the
reasoning for keeping them as a future option should be restated.
