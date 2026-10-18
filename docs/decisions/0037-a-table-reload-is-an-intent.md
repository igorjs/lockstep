<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0037 A table reload is an intent (decided)

Status: decided and implemented in milestone M16.

## Decision

- A table is replaced while a session runs by an intent, `TableSwap`, which carries the table's
  name and the new JSON text. The simulation applies it with `Table::swap` at a step boundary,
  like every other intent.
- The intent carries the text, not a file name or a path, so a recording holds everything a replay
  needs. The host may read a file, watch it for changes and send the text; the framework never
  touches a file.
- Which table a swap names, and what a refusal looks like as an event, belong to the simulation.
  The framework gives the intent's shape and the check; the consumer routes it.
- A swap is bounded (`MAXIMUM_SWAP_BYTES`, one mebibyte) because it lives in the recording.

## Alternatives rejected

- The host replacing the table between steps, outside the recording: a replay would run every
  step on the first table and diverge at the first checkpoint after the change.
- A swap that names a file: a replay on another machine, or a day later, would read a different
  file.
- Applying a swap at a time the host chooses within a step: steps are the only moments the
  state changes.

## Would change if

`a_reload_is_an_intent.rs` fails: a recording with a swap must replay to the live hash, and the
same recording without its swap intent must diverge.
