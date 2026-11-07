<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0082 An exact precomputed abstraction does not meet the thresholds (decided)

Status: NO-GO. `Pathfinder` stays the default exact search. Gap G2 is closed without a new search.

## Decision

A measurement spike asked whether an exact abstraction, one that returns the cost of `Pathfinder`, could answer 500 paths on the benchmark map (512 by 512, 20 percent walls, seeded as in `benches/spatial.rs`) in under 50 milliseconds on one thread, with a build under 5 seconds. It cannot, so no abstraction is built and none enters the public interface.

The prototype was a contraction hierarchy on the 209,482 passable cells: nodes contracted by lazily updated edge difference plus deleted neighbours, with a witness search limited to 60 settled nodes (a limit only adds shortcuts, so it stays exact), and a bidirectional upward Dijkstra for queries. It assumed symmetric steps, which holds on the flat benchmark map. The prototype was throwaway code and is not committed.

## Numbers

Release build, one thread, on the machine of `benches/baseline.txt`, not quiet (load average 3 to 6 from other agents). Three runs:

| Measure (seconds for the build, milliseconds for the paths) | Run 1 | Run 2 | Run 3 | Limit |
|---|---|---|---|---|
| Build, seconds | 18.8 | 19.2 | 22.3 | 5 |
| 500 paths, best of five, milliseconds | 163 | 193 | 214 (worst single runs 590 under load) | 50 |

The hierarchy held 521,839 shortcuts and 1,054,530 upward edges (about 5 per cell). The costs equalled `Pathfinder` on all 500 pairs (zero mismatches). The query time is for the cost only, so unpacking a path to cells would add to it. Both limits are missed: the build by about four times, the query by about three times.

## Why

A grid with random walls has no hierarchy of important roads. Contraction goes fast while the graph is sparse (150,000 cells in 1.6 seconds), then the last 10,000 cells form a dense core whose shortcuts grow with the width of the map, so the build and the searches in the core dominate. The query settles thousands of core nodes per path; the jump batch and the hierarchical search already beat that by cutting the problem differently.

## Alternatives

- Tune the witness limit, the priority weights or stop contracting at a core: each trades build time for query time, and a threefold query gain with no build loss is not plausible. This is an estimate, not a measurement. Not pursued.
- A GO would have rebuilt on `GridMap::revision()` change and reached Segment S6. No S6 is needed.

## Tests

`tests/decisions/exact_search_stays_pathfinder.rs`: a seeded property test (seed 82, 40 maps of 16 by 16 with terrain cost and walls, 20 queries each) that `Pathfinder` matches a plain Dijkstra on cost, and that `JumpPathfinder::new` declines the ground. No timing is asserted. No fixture hash changed.
