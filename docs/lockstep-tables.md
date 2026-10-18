<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-tables

Data tables: named rows read from JSON, checked as a whole, and replaced while a session runs. The
crate knows nothing about what a row means; a simulation chooses its row type and what a swap
does to the rest of its state.

## Tables

`Table<Row>` holds rows by name in name order. `Row` implements `TableRow`, whose `check` rejects a
row that is well formed but wrong (a price below zero, say). `Table::from_json(text)` reads

```json
{ "rows": [
  { "name": "economy", "base_minor": 500, "per_kilogram_minor": 120 },
  { "name": "express", "base_minor": 900, "per_kilogram_minor": 260 }
] }
```

where each row is its name beside the row's own fields. It refuses text that is not JSON of this
shape, a table of no rows, a repeated name, more than `MAXIMUM_ROWS` rows, text longer than
`MAXIMUM_SWAP_BYTES`, and any row whose `check` fails. `Table` serializes as its rows and
deserializes through the same checks, so a damaged save is refused too (decision 0038).

`digest()` is a number that names the table's content: the order of the file does not change it.

## Swapping

`Table::swap(text)` builds the new table, and only if it passes replaces the old one. It returns
`Swapped::Replaced { from, to, rows }` with the two digests, or `Swapped::Unchanged` when the
content is the same, or the error with the table untouched.

`TableSwap { table, json }` is the intent that carries a swap (decision 0037). A simulation keeps
its table in its state, adds a variant holding a `TableSwap` to its intent type, and on that intent
checks the name and calls `swap`, then reports `Replaced`, `Unchanged` or the refusal as events.
The host's part is outside the framework: read the file, watch it, send the text as an intent.
A recording holds the text, so a replay swaps at the same step.

## Consumer

[rates](rates.md): a parcel desk quoting from a rate table that is swapped four times in a
session, once with a broken table, once with the same content and once dropping a service.
