<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0012 The timeline is a projection (decided)

Status: decided and implemented in milestone M5.

## Decision

The timeline is an append-only log of events with indexes by entity and kind, kept outside the
snapshot and the state hash, and rebuilt from a recording's inputs whenever it is needed. Entry
times are the clock's whole minute of the day, an integer from its exact position. Compaction is
the caller's summariser, applied to everything before a step.

## Alternatives rejected

- Keeping the event log in the snapshot: saves and hashes grow with history, and a journal format
  change becomes a save migration.
- Times from the float `minute_of_day`: its last bits can differ between platforms.
- A fixed compaction rule in the core: what to keep (a daily total, a count per kind) belongs to the
  consumer.

## Would change if

Keeping a timeline changes a runner's hash, or a timeline rebuilt from a recording differs from the
live one (`tests/decisions/timeline_is_a_projection.rs`); or compaction loses counts it summarises
(`tests/behaviour/timeline.rs`).
