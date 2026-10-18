<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0043 Every example with one session is recordable (decided)

Status: decided and implemented in the headless runner.

## Decision

- An example is recordable when its fixture is one session on one runner. Each recordable example
  writes its script once, against `Session`, which a runner and a recorder both implement, so the
  fixture hash and the recording come from the same loop.
- The headless table is one line per example, expanded from a macro. An example joins by exporting
  `SIMULATION_ID` and `record_fixture`.
- `mars-rovers` and `orchard` are excused. The first runs many separate sessions in one fixture.
  The second sets the clock's daylight from outside the simulation between steps, which a replay
  does not do, so its recording would diverge.

## Alternatives rejected

- A copy of each script for recording: the two could drift, and the recording would stop matching
  the fixture hash.
- Recording by observing a runner afterwards: a runner keeps no intents.
- Moving orchard's daylight into its simulation: it changes a fixture hash, which is a separate
  decision for its owner.

## Would change if

`every_fixture_is_recordable_or_names_why_not` fails: a fixture is neither in the table nor in
its list of excused examples, or is in both.
`every_recording_replays_to_its_fixture_hash` fails: a recording and its fixture disagree.
