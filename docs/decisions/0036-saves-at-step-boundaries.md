<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0036 A save is taken at a step boundary, and a recording continues from it (decided)

Status: decided and implemented in `lockstep-core` in milestone M16. The headless commands follow in the next pull request.

## Decision

A save holds the state the runner hash covers and nothing else: the step number, the clock, the
streams and the snapshot. It does not hold the accumulator of unspent real time or the intents
queued for the next step. Neither is in the hash, and a replay uses whole steps, so a session
continued from a save sees what the original saw.

`replay_from` continues a recording from a save: it skips the steps before the save's step and
compares every checkpoint from that step on. The save's own hash is compared first: against the
checkpoint at its step when there is one, and otherwise against the recording's steps run from the
start to that step, so a save between checkpoints is still checked. A save taken with other step or
clock settings than the recording is refused.

- `Recorder::save` takes the save of a session in progress. `lockstep-headless replay <file> --until
  <step> --save <file>` writes one from a recording, and `resume <save> --recording <file>` continues.
  This closes deferral D3.
- A save converted from an older snapshot shape cannot be compared with a recording made by the
  current shape: the recorded hashes cover the shape that wrote them. `replay_from` says so
  (`NotComparable`) instead of passing vacuously. Such a save is checked by a session run on the
  current code from the same start.

## Alternatives rejected

- Saving the accumulator and queued intents: a save taken mid-frame would differ from one taken at
  the same step a frame later, and the difference would be invisible to the hash.
- Comparing only the checkpoints after the save: a save from the wrong run would pass until its first
  later checkpoint, and a save at a checkpoint step would never be checked.
- Treating a converted save as identical when nothing diverges: no hash is compared, so it would be a
  pass that checked nothing.

## Would change if

Loading a save changes the hash at any step, or a save that differs from the recording at its own
step continues without a divergence.
