<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0055 A wave falls due by simulated minutes (decided)

Status: decided and implemented in milestone M8, closing deferral D5.

## Decision

- A wave is data: a total, and rules of pulse size, interval in simulated minutes and a start delay.
- The pulses due are counted from the simulated minutes since the wave began, not step by step, so
  the same simulated time gives the same pulses at any step rate. The time is kept as raw 16.16
  minutes from the start and never reset, so it does not drift.
- The last pulse is the remainder of the total.
- The state is plain data, so a snapshot taken mid-wave restores and continues exactly.

## Alternatives rejected

- A pulse every so many steps: the arrivals would depend on the frame rate.
- Carrying a remainder after each pulse: the same count, but a rounding error in each step
  would add up over a long wave.

## Would change if

`waves_follow_simulated_minutes.rs` fails: a wave stepped in whole, half and quarter minutes differs
in when pulses fall due, in `crates/lockstep-agents/tests/decisions/`.
