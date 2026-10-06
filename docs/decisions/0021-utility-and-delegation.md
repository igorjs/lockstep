<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0021 Choices are integer sums, and a refusal names its weakest reason (decided)

Status: decided and implemented in milestone M8.

## Decision

- A choice scores the integer sum of its considerations, added in 64 bits. The highest total
  wins; a tie goes to the lowest id.
- A delegated task is accepted at or above one total, delayed at or above a lower one, and
  otherwise refused. A refusal names the consideration that scored lowest, the first listed on a
  tie.
- Considerations are plain functions of an agent and a world type the simulation chooses, so
  relations, attributes and memory all feed them without this crate knowing about them.

## Alternatives rejected

- Weighted floating point scores, or learned ones: they differ across platforms. Tuning happens
  offline from headless runs, as data.
- A refusal with no reason: the host has nothing to show the player.
- A fixed world view trait: every simulation would have to fit its shape.

## Would change if

`crates/lockstep-agents/tests/decisions/utility_ties_and_refusals.rs` fails: two choices of equal
total pick the higher id, or a refused task names anything but its lowest-scoring consideration.
