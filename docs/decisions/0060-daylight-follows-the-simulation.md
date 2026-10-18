<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0060 Daylight follows the simulation (decided)

Status: decided and implemented in milestone M16.

## Decision

`Simulation::daylight(&self, day) -> Option<(sunrise, sunset)>` is asked by the runner before every
step, with the clock's current day. A `Some` is set on the clock; the default `None` leaves the
clock as it is. The orchard answers from its calendar. No recording holds daylight, because the
simulation that makes the answer is the one a replay runs.

This moves the orchard's per-step call from the host into the simulation, which decision 0043 had
set aside as a change to a fixture hash. It is not: the runner asks at the same point the host did
(after the previous step, before the clock advances) with the same day, so the orchard fixture hash
is unchanged.

## Alternatives rejected

- Daylight in every recorded step: it repeats a value the simulation already knows and changes the
  layout of every recording.
- A hook that gets the whole clock: a simulation could then move time itself, which only the runner
  may do.
- Leaving the host to set it and refusing to record the orchard: the gap this closes.

## Would change if

`daylight_follows_the_simulation` fails: the runner sets another day's daylight than the clock's
current day, or a simulation that returns `None` has its clock changed.
