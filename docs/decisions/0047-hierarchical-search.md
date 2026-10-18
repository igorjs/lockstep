<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0047 Hierarchical search for long paths (decided)

Status: decided and implemented in `lockstep-spatial`. It answers the pathfinding benchmark target left open in decision 0005 (gap G2).

## The problem

The reference asks for 500 paths on a 512 by 512 map with 20 percent walls in under 50 milliseconds on the development MacBook. The exact search (`Pathfinder`, A* keyed by `(cost, cell)`) takes about 611 milliseconds there: about 12,700 cells expanded per path, for paths of about 266 cells.

## What was measured, and what each option gave

All on the benchmark map (`cargo bench -p lockstep-spatial`), Apple M1 Pro, release profile, best of five. The machine was shared with other builds while these were taken, so the exact search read between 570 and 700 milliseconds across runs; compare the ratios, not the last digit.

| Option | Result | Kept |
| --- | --- | --- |
| Pack the search state of a cell into one record and the heap key into one word | exact search 570 to 580 milliseconds instead of 610 to 700 in the same session; same paths | no (about 7 percent, not worth a second implementation of the search) |
| Step without division (neighbour offsets, corner cells and step costs computed once per cell) | no measurable change | no |
| Break heap ties by remaining distance instead of by cell | 6.29 million expansions instead of 6.35 million; also changes the tie-break of decision 0005 | no |
| Landmark estimates (distance to 4, 8, 16 spread cells), search cost identical to the exact one | 4.8, 4.4 and 4.2 million expansions instead of 6.35 million; building them took 80 to 280 milliseconds | no |
| Jump point search | not built: it needs uniform terrain cost and flat ground, and `GridMap` has cost and elevation per cell | no |
| Hierarchy of chunk entrances (this decision) | 22 milliseconds for the 500 paths, paths 6.36 percent dearer in total | yes |

The expansions do not come from ties or from cost of a step. They come from the straight-line estimate being a poor guide on a map with many walls: the cheapest path costs about 1.15 times the estimate, so the search must expand everything inside a wide ellipse. Only a search that does not look at those cells, or an estimate close to the truth, removes them.

## Decision

`Hierarchy::build(map, side)` cuts the map into chunks of `side` cells and finds the entrances: for each run of open cells along a chunk border, one entrance (the middle of a short run, both ends of a long one), as a pair of cells that a body can step between both ways. Inside each chunk, it keeps the cost and the route between every two entrances, minus the edges that a route through a third entrance matches (which keeps every distance and shrinks the graph several times).

`HierarchicalPathfinder::find` then:

1. searches inside the start chunk and inside the target chunk (a whole-chunk search each);
2. searches the entrance graph with A* between them;
3. copies the stored routes of the entrance graph edges into the path.

Answers are integer arithmetic over fixed orders, so they are the same on every platform (the committed hash `fixtures/hierarchical_paths.hash` has a WebAssembly twin).

`Pathfinder` stays exactly as it was and stays the default. Nothing that uses it changes, so no existing fixture hash changes. The hierarchy is opt-in, for long paths on large maps where the cheapest path is not required.

### What it costs, said plainly

- **Paths are dearer.** A path crosses chunk borders at entrances only. On the benchmark map the total is 6.36 percent above the exact total with the default greed; 3.88 percent at greed 1.1 (42 milliseconds) and 2.76 percent at greed 1.0 (106 milliseconds). On the hilly and muddy maps of the decision test (one-way drops everywhere) it is 13.2 percent.
- **Greed.** The entrance search multiplies its estimate by `greed` tenths (default 12, so 1.2). Above 1.0 the estimate may overshoot the true cost, so the search commits sooner and expands far fewer entrances, at the price of dearer paths. `set_greed(10)` gives the plain estimate.
- **Chunk width.** Sixteen won the sweep below. Eight gives twice the entrances and, at the same greed, paths 2 percentage points dearer; thirty-two spends its time searching whole chunks at both ends.
- **A build.** About 230 milliseconds for the benchmark map, once per map. A change to the map makes the hierarchy stale (`GridMap::revision` counts changes; it is not saved, hashed or compared). A search given a stale hierarchy is the exact search, correct but slow, so rebuild after changes.
- **Exact search in these cases.** Bodies counted as walls, an expansion limit, an end that is a wall, a stale hierarchy, or a border that can only be crossed one way (a drop) leaving the hierarchy without a route. In each the answer is exactly the answer of `Pathfinder::find`.
- **Unreachable.** A target walled into its chunk, or a start walled into its chunk, is answered `Unreachable` without searching the map. Any other failure goes to the exact search, so reachability always agrees with `Pathfinder`.
- **No batch yet.** `find_paths` still runs the exact search. A batch of hierarchical searches would give each thread its own `HierarchicalPathfinder`; it waits for a consumer that needs it.

## Measured

Sweep on the benchmark map (500 paths, one of them unreachable), time to search once the hierarchy is built, extra cost in total over the exact paths:

| Chunk side | Greed | Entrances | Milliseconds | Dearer |
| --- | --- | --- | --- | --- |
| 8 | 1.0 | 35,441 | 270 | 5.18 percent |
| 8 | 1.2 | 35,441 | 20 | 8.32 percent |
| 16 | 1.0 | 16,814 | 106 | 2.76 percent |
| 16 | 1.1 | 16,814 | 42 | 3.88 percent |
| **16** | **1.2** | **16,814** | **23** | **6.36 percent** |
| 16 | 1.5 | 16,814 | 20 | 9.80 percent |
| 24 | 1.2 | 11,208 | 38 | 5.60 percent |
| 32 | 1.0 | 8,042 | 117 | 1.38 percent |
| 32 | 1.2 | 8,042 | 63 | 4.91 percent |

(`cargo bench -p lockstep-spatial -- --sweep` prints the whole grid.) Committed in `benches/baseline.txt`: `hierarchy_build` and `paths_500_hierarchical`.

The reference's target of 50 milliseconds is met by the hierarchy and not by the exact search: the exact serial search still takes about 611 milliseconds (88 on the path batch with ten threads). The gap that remains is for exact search, and it stays recorded in the roadmap.

## Alternatives

- Make the exact search faster until it reaches 50 milliseconds: every cheap change measured above moved it by under 10 percent, and 12 times fewer expansions needs a different search.
- Two-level or multi-level hierarchy, bidirectional search: not built or measured. With the default greed the search over entrances expands about 90 entrances per path, against about 450 cells for the two whole-chunk searches, so a second level has little left to save.
- Smoothing the path after the search to claw back cost: not built.

## Tests

`tests/behaviour/hierarchy.rs`: valid paths and reachability agreeing with the exact search on `Square8`, `Square4` and hexagons; an open map within 10 percent of the cheapest; walled-in targets and starts; a stale hierarchy, bodies as walls and an expansion limit each giving the exact answer; more greed searching less and never finding a cheaper total; two builds searching the same paths. `tests/decisions/hierarchical_search_for_long_paths.rs`: the committed path hash, and the cost against work measure (at most 15 percent dearer, at least four times fewer expansions). Each was shown to fail when its rule was removed. The consumer scenario is `examples/crowd/tests/long_routes.rs`: eighty couriers on a 160 by 160 map.
