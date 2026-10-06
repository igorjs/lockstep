<!-- SPDX-License-Identifier: Apache-2.0 -->

# crowd (example)

The consumer scenario for the path batch (decision 0006).

- 60 bodies start on a 48 by 48 map with random walls and walk to one goal cell.
- Every step, the bodies still walking ask for their paths in one `find_paths` batch, with the other bodies treated as walls.
- Answers apply in handle order through occupancy, so a body sees the moves of lower handles in the same step. A body that reaches the goal leaves the map.

The fixture hash is committed in `fixtures/crowd.hash`. `just determinism` verifies it twice: serially, and with
`--features parallel`, where the batch runs on a thread pool. Both must give the same hash.
