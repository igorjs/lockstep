<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0026 Inputs go all or none, and an outcome is one weighted draw (decided)

Status: decided and implemented in milestone M12.

## Decision

- A recipe consumes all its inputs or none. Every input is counted first, across as many stacks
  as hold the kind and leaving spoiled units out; only when all are there is anything consumed.
- An outcome is one number drawn from the `"crafting"` stream below the table's total weight,
  taking the row whose running total passes it. Each row's share converges on its weight.
- A job takes whole game minutes at its station, one job a station at a time, and its outputs
  go into the station's container, or are left loose when there is no room.

## Alternatives rejected

- Consuming input by input and stopping at the first short one: the inputs already taken are lost.
- One roll per row in turn: it skews toward the rows listed first.
- Dropping outputs that do not fit: the work would vanish.

## Would change if

`crates/lockstep-crafting/tests/decisions/` fails: a mixer short of flour takes any water, two
flour from two one-unit stacks are not taken, spoiled flour counts, or 100,000 draws of 85, 10
and 5 land outside half a percent of each weight.
