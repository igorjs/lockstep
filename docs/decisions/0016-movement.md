<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0016 Movement answers on its own step, and noise doubles when running (decided)

Status: decided and implemented in milestone M6.

## Decision

- A move order takes its first cell on the step it arrives: the first cell's cost is paid up front,
  and later cells wait for progress to add up. The reference asks for one step from click to
  motion.
- Progress adds up exactly: speeds in cells a second against the steps per second, in integers,
  with a diagonal step costing 1.4 cells.
- Stamina is the `Fighter`'s, so attacks, dodges and running spend one pool. A body without a
  fighter never tires.
- Each step is heard at the cell's walking distance, times two when running and one half when
  sneaking, so running is heard at exactly twice the walking distance.
- A blocked step tries the two neighbours beside the intended direction, then after half a second
  paths again around bodies.

## Alternatives rejected

- Moving only once a full cell of progress builds up: several steps of delay on every click.
- A separate stamina pool for movement: a fighter could run on empty and still dodge.
- Waiting forever behind a body: crowds lock up in corridors.

## Would change if

`crates/lockstep-combat/tests/decisions/movement_feel.rs` fails: motion on the order's own step,
running at exactly twice the walking noise, and stamina empty after 16.5 to 17 seconds of running.
