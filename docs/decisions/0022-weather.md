<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0022 Weather advances by whole game seconds (decided)

Status: decided and implemented in milestone M8.

## Decision

- The weather advances one whole game second at a time, carrying any part of a second to the
  next step, and draws from the `"weather"` stream only on those seconds. The same game minutes
  give the same weather at any step rate.
- Outside a front the direction drifts at most 12 turn units a second, under the reference's 4
  degrees a game minute. A front swings 90 to 180 degrees over 8 to 12 minutes on a straight
  schedule. The strength closes a thirtieth of its gap to the mean each minute. Gusts add 50 to
  100 percent for 30 to 90 seconds, 1.5 times an hour on average.
- The weather lives in `lockstep-agents`, beside the wind it drives, until the regions crate
  exists (departure P18).

## Alternatives rejected

- Drawing once per step: the weather would depend on the frame rate.
- Drawing once per game minute: gusts of 30 to 90 seconds need a finer tick.

## Would change if

`crates/lockstep-agents/tests/decisions/weather_from_seed_42.rs` fails: a day at 30 and at 60
steps a second differs, or the seed 42 hash changes without a commit naming the weather rule
that changed.
