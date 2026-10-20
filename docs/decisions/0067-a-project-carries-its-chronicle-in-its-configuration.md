<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0067 Successive projects record as a set, each carrying its chronicle in its configuration (decided)

Status: decided and implemented in milestone M16. It closes gap G4.

## Decision

`roster` records as a `RecordingSet` (decision 0061) of two sessions, labelled `first` and `second`,
in the order the fixture ran them. The second project starts from a chronicle folded from the
first project's events. That chronicle reaches the simulation only through its `Configuration`
(the retired names), so the second session's recorded configuration already holds it. A replay
runs each session from its own recorded configuration and needs no chronicle and no events from
the first.

- The fixture and the recording share one loop (`record_projects`), as in decision 0043. The
  chronicle still goes through `seal` and `open` between the projects.
- The fixture hash is now the hash of the two projects' final hashes, because that is what a
  replay of the set reproduces. It no longer covers the chronicle itself. The retired names that
  matter reach the second project through its configuration, so they are still in the hash.
  The fixture changed (`8969859445141e9b` to `b8fdc6abe1eaee12`, together with decision 0068).
- `replay`, `bisect` and `stats` work on the set; `replay --save` and `resume` are refused, as for
  every set.

## Alternatives rejected

- Put the first project's events in the recording and refold on replay: the recording grows by
  every event, the replay needs code the simulation does not have, and a project's configuration
  is already the input the simulation takes.
- Keep `roster` excused: a gap that a recording set closes without changing the simulation.
- One recording that continues the first project into the second: the second is a new simulation
  with a new seed, not more steps.

## Would change if

`the_recorded_set_replays_both_projects_to_the_fixture_hash` or
`every_recording_replays_to_its_fixture_hash` fails: a project depends on something a recording
does not carry.
