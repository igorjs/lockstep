# 0005 The spatial crate (decided, with one open item)

Status: decided and implemented for the first part of `lockstep-spatial` (topologies and the map),
with one open item (the hexagon figure). The reference calls this crate `lockstep-grid`;
it was renamed to stay generic.

## Decisions where the reference was silent or ambiguous

- **Directions.** North is a smaller y, and `Square8` lists neighbours clockwise from north. Hex lists them clockwise from north-east.
- **`neighbours` and `line` clear their output first**, so a caller can reuse one buffer.
- **`step_cost` equals `distance`** for adjacent cells: 10 straight, 14 diagonal.
- **Climbing.** `step_limit` bounds a climb only. A drop of any height is allowed.
- **No corner cutting.** A diagonal step is refused when either orthogonal neighbour is a wall. The test is topology-neutral: a step that costs more than a straight step is a diagonal.
- **Walls.** The reference says walls block "as elevation + 255" and that a low wall "blocks only if it reaches the interpolated eye line". Both are kept with one extra byte per cell, `wall_height`: `set_passable(cell, false)` is a full wall of 255 on top of the cell's elevation, and `set_low_wall(cell, height)` is a lower wall. Line of sight (next part) compares the top of each cell with the eye line.
- **`hex` is a feature.** It is not enabled by default. `just test` and `just check` also run the crate with `--features hex`.

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
