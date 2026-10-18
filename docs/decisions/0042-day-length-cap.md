<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0042 A day takes at most 2^32 steps (decided)

Status: decided and implemented in milestone M16, found by fuzzing.

## Decision

`Clock::new` caps the steps in a day at 2^32 (over two years at 60 steps a second). A day length
or step length that asks for more, such as a damaged recording with an astronomical day or a step
of zero seconds, gets exactly 2^32. Before this, the day's 64-bit units overflowed and the clock
panicked in debug builds and wrapped in release builds, so a damaged recording could crash a
replay. Every product of a position and a constant in the clock now fits in 64 bits.

## Alternatives rejected

- An error from `Clock::new` and `Runner::new`: a changed signature for a case no real session
  reaches.
- Checked arithmetic throughout the clock: a cost on every step to guard a configuration.

## Would change if

`a_day_has_at_most_four_billion_steps.rs` fails: a design needs a longer day, or a configuration
below the cap changes its position.
