<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0038 A swap is all or nothing, and the table is state (decided)

Status: decided and implemented in milestone M16.

## Decision

- A table is part of the simulation's state: it is in the snapshot, so it is in the hash and in
  every save. A save taken after a swap restores the swapped table.
- A swap checks the whole new table before it applies anything: the JSON shape, at least one row,
  no repeated name, the row limit, and each row's own `check`. On any fault the old table stays
  exactly as it was and the simulation reports a refusal.
- A decoded table runs the same checks, so a damaged save cannot carry a table that never passed.
- Rows are kept by name in name order, so the order of the file never changes the content.
  `digest` names the content; a swap whose digest equals the current one changes nothing
  (`Swapped::Unchanged`).

## Alternatives rejected

- Applying the rows that pass and skipping the rest: the result is a table no file describes.
- Keeping the table outside the snapshot and re-reading it on load: a save would run on whatever
  the file holds today, not on what it was saved with.
- Comparing text instead of content: a reordered or reformatted file would count as a change.

## Would change if

`a_swap_is_all_or_nothing.rs` fails: a rejected swap must leave the state hash as it was, and a
restored save must run on the table it was saved with.
