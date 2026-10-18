<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0061 A fixture of many sessions records as a set (decided)

Status: decided and implemented in milestone M16.

## Decision

A fixture that runs several separate sessions records as a `RecordingSet`: the simulation id first,
as in a `Recording`, then each session as a label and a recording, in the order the fixture ran
them. The label tells a tool which variant of the simulation runs the session (the topology, for
`mars-rovers`), because one simulation type cannot decode them all.

- `replay` runs every session and prints `identical <id> after <n> sessions of <steps> steps, hash
  <hash>`. The hash is the fixture's: the hash of the sessions' final hashes, in order. A
  divergence names the session and its label.
- `bisect` narrows the first session that differs, and says which one.
- `stats` totals the sessions, steps, intents and checkpoints.
- `replay --save` and `resume` are refused: a set has no one state to save.

The fixture and the recording share one list of sessions and one loop, so they cannot drift
(decision 0043).

## Alternatives rejected

- One recording that concatenates the sessions: the sessions start from different configurations
  and topologies, so one runner cannot hold them.
- One file per session: the fixture hash covers all of them, and a replay should reproduce it from
  one file.
- A separate simulation id per topology: three recorded names for one example, and no single
  fixture to match.

## Would change if

`every_recording_replays_to_its_fixture_hash` fails for a set, or a divergence in one session is
reported against another.
