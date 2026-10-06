<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0011 Recordings hold inputs and checkpoint hashes (decided)

Status: decided and implemented in milestone M5.

## Decision

A recording holds the starting point (simulation id, configuration bytes, seed, step and clock
configuration), every step's intents with the clock multiplier the host had set, and the runner
hash at step 0 and every `checkpoint_every` steps. Replay runs the inputs again and compares hashes
at the checkpoints; `bisect` names the last matching and first differing checkpoint.

- The multiplier is recorded exactly as the host passed it, so the replay sets the same fixed point
  rate even where the float the clock reports back would round.
- The recorder sees every step `advance` runs, through a hook in the runner, so a checkpoint inside
  a frame that ran several steps is still taken.
- Snapshots at checkpoints are opt-in (`keep_snapshots`). With them, `bisect` lists the lines of the
  recorded and replayed snapshots that differ. The spec's `Recording` has no snapshots; this is the
  addition that lets bisect show what changed instead of only where.
- `bisect` narrows to the checkpoint, not the step: the recording has no hash between checkpoints.
  Checkpoints every step name the exact step, at a hash per step.

- A recording's inputs are the intents and the clock multiplier, nothing else. Anything else that
  changes what a step sees, such as daylight, must come from the simulation's own rules (the
  calendar computes it from the day), never from a host call the recording cannot see.
- A malformed recording fails loudly: a checkpoint out of order or past the last step is an error,
  a kept snapshot that no longer decodes is an error, and `to_bytes` refuses a recording too large
  to read back.

## Alternatives rejected

- Recording every step's state: a ten hour session grows from megabytes to gigabytes.
- Recording events: a rule change changes them too, so they cannot tell a divergence apart from a
  feature.
- Replaying with `advance` and recorded frame times: the float accumulator would have to match bit
  for bit; recording the steps the frames produced removes it.

## Would change if

The decision test plants a rule change at step 1,234 and must find it between checkpoints 1,200 and
1,300, at exactly step 1,235 with checkpoints every step, and fuzzed sessions of frames, queues and
multipliers must replay to the same hash.
