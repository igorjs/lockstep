<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0081 A bucket open set does not pay, an early stop barely does (decided)

Status: gap G2 stays open for a single thread.

## Decision

Two cheaper forms of the jump search (decision 0075) were tried on the benchmark map (500 paths, 512 by 512, 20 percent walls).

- A bucket open set, one bucket per estimate with the cells of a bucket ordered. With a small heap per bucket the serial run took about 366 milliseconds against 326 for the binary heap on the same machine and day. With an unordered bucket and a scan for the smallest cell it took about 550. The estimates are flat across a wide ellipse, so each bucket holds many jump points. Rejected, not kept.
- An early stop: when the goal has a cost, a popped jump point whose estimate is not below that cost ends the search with the goal. This is exact: the estimate is consistent, so nothing left can be cheaper. Kept. It cuts expansions from 173,847 to 173,667 on the seeded maps of the decision test (0.1 percent) and costs nothing per step.
- A check on every push against the goal's cost was also tried. It adds a distance computation per push and saved no further expansion. Rejected.

The expansions are deterministic, so the test asserts them; the time is only in `benches/baseline.txt`.

## Numbers

Committed baseline from this machine (microseconds): plain 694,226, jump 343,654, jump batch 72,138, hierarchical 23,068. The machine is slower and noisier than the one behind the earlier numbers (jump 317,192, batch 45,695). The 50 millisecond target is not met by the serial search, and the batch is above it here.

## Tests

`tests/decisions/early_stop_for_the_jump_search.rs`: a seeded property test (seed 81, 120 maps, 25 queries each) that the result and cost equal `Pathfinder`, and a deterministic expansions assertion that fails without the stop. `paths.hash` and `hierarchical_paths.hash` are unchanged. The consumers `crowd` and `mars-rovers` pass.
