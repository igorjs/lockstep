<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0075 Jump search is exact and opt-in (decided)

Status: decided and implemented in `lockstep-spatial` (`JumpPathfinder`). It is a partial answer to gap G2: exact, faster, not fast enough.

## Decision

On `Square8` maps whose passable cells all have normal terrain cost (1) and one elevation, `JumpPathfinder` runs jump point search without corner cutting (a diagonal step needs both side cells open, as `GridMap::can_step` says). It returns the same `PathResult` as `Pathfinder` for every query with an unlimited expansion budget, including the same cost, and a walkable path.

`Pathfinder` stays the default. The two searches may choose different cells among paths of equal cost: the plain search breaks ties by cell index, while the jumps skip the cells that decide that tie break. To keep every fixture hash unchanged, the jump search is a separate type, and a fixture made with one search is not valid for the other. `JumpPathfinder::new` returns `None` for any other ground, and a search that counts bodies as walls, or starts inside a wall, is handed to the plain search. `maximum_expansions` counts jump points. A map that changed since construction panics (rebuild).

## Measured

Benchmark map (500 paths, 512 by 512, 20 percent walls), Apple M1 Pro, release, best of five, machine shared with other builds:

| Search | Microseconds | Expanded |
| --- | --- | --- |
| Plain A* | 611,000 to 690,000 | 6.35 million cells |
| Jump search, byte grid | 379,000 | 2.93 million jump points |
| Jump search, 64 cell word runs | 371,000 | 2.93 million jump points |

The word runs removed no measurable time: with 20 percent random walls nearly every cell has a forced neighbour, so runs are short and the cost is in the 5,900 jump points per path, not in scanning. The total cost over the 500 paths is asserted equal to the plain search in the benchmark.

## Alternatives

- Make it the default: changes paths and fixture hashes. Rejected.
- Match the plain search cell for cell: needs the cells the jumps skip. Rejected.
- Bucket open set, bidirectional search, a per-map abstraction (contraction hierarchy, hub labels): not built. Decision 0076 states what 50 milliseconds would need.

## Tests

`tests/decisions/jump_search_keeps_the_cost.rs`: 12,000 seeded queries over 400 random maps (some wider than one 64 cell word) give the plain search's result and cost with a walkable path; non-uniform ground is refused; bodies as walls and wall starts agree with the plain search; a changed map panics; fewer expansions than the plain search. Shown to fail when one side of the forced neighbour test was removed. Consumer scenario: `examples/crowd/tests/long_routes.rs`. No fixture hash is added, so there is no WebAssembly twin; the decision tests run under WebAssembly too.
