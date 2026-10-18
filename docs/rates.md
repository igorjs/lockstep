<!-- SPDX-License-Identifier: Apache-2.0 -->

# rates (example)

A consumer scenario that is not a game: a parcel desk quotes prices from a rate table while the
table is swapped. It shows data tables with hot reload ([lockstep-tables](lockstep-tables.md)).

- A quote is the service's base price plus its price per started kilogram. An unknown service or a
  weight of nothing is refused.
- The `SwapRates` intent carries a `TableSwap`. A table that fails a check, is not JSON, or names
  another table is refused with `SwapRefused` and the rates stay. The same content in another order
  gives `RatesUnchanged`. A good table gives `RatesSwapped` and prices the next quote.
- The scripted session quotes once a step and sends four swaps: a rise at step 1,000, a table with a
  negative price at 2,000 (refused), the rise again at 2,500 (unchanged) and a table without
  freight at 3,000, after which freight quotes are refused.
- A recording of the session holds all four swaps and replays identically.

## Fixtures

`fixtures/rates.hash`: the scripted session, 4,000 steps from seed 20,261,020. It is checked natively
and under WebAssembly.

## Headless

`lockstep-headless` verifies, records and replays `rates`. See [lockstep-headless](lockstep-headless.md).
