<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0079 Hash every step, keep snapshots coarse (decided)

Status: decided and implemented. It narrows gap G3.

## Decision

- `Bisection` gains `exact_step`, the index of the step whose execution first changed the state (`first_bad_step - 1`). It is set only when the last good and first bad checkpoints are adjacent, so it is `None` for a change in `create` and for recordings with wider checkpoints. `checkpoint_interval()` gives the width then. The change is additive: `last_good_step` and `first_bad_step` mean what they did.
- `Recorder::snapshot_every(n)` separates the snapshot spacing from the checkpoint spacing. The default keeps a snapshot at every checkpoint, so existing recordings and callers are unchanged.
- `lockstep-headless record` defaults `--every` to 1 and keeps snapshots every 100 steps (a multiple of `--every`). It records the deterministic session twice, once for hashes and once for the snapshots, so the `record_fixture` signature in every example stays as it is.
- `bisect` prints `exact step: <n>`, or a hint to re-record with `--every 1` when the checkpoints were wider.

## Why

A checkpoint is one 16 byte pair, so a hash at every step is cheap; a snapshot per step is not and could reach the decode limit (decision 0011 rejects per-step state). Measured at the default step counts, the largest recording is 2.9 MB (crater-survey), orchard at 8,000 steps is 1.2 MB, and the limit is 256 MiB.

## Departure

`Recorder::snapshot_every` has no example consumer: the headless layer records twice to keep every `record_fixture` signature unchanged. Passing the option through the example crates is a later change if the double recording cost matters.

## Costs

Where no snapshot was kept at the bad step, `bisect` shows no differing lines. That loss of detail is accepted and tested.

## Would change if

A recording of a real session nears the decode limit with a hash at every step.
