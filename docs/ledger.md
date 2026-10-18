<!-- SPDX-License-Identifier: Apache-2.0 -->

# ledger (example)

The consumer scenario that is not a game: accounts and transfers.

- Balances are whole minor units (`i64`) in one currency.
- A transfer moves exactly the amount or is rejected with a reason: `UnknownAccount`, `NotPositive`, `SameAccount`, `InsufficientFunds`, `Overflow`. Nothing panics and nothing changes on a rejection.
- Money is only created by deposits, so the sum of balances is the opening total plus deposits.

The fixture draws one random intent per step from its own stream set. Its hash is committed in
`fixtures/ledger.hash`.

## Recording and journal

`record_fixture(seed, steps, checkpoint_every)` records the scripted session with a `Recorder`.
It replays to the same hash as the fixture run, and `Timeline::rebuild_from::<Ledger>` rebuilds its
events. `journal(&timeline, &books)` renders them as an audit log, one line per event:

```text
day 0 00:00  #0      312.45 moved from account-3 to account-6
day 0 00:00  #1      rejected: InsufficientFunds
```

The tests check that the journal of a replay matches the live session line for line, and that the
timeline's counts agree with the books.

## Shared bills

The `bills` module uses the core's `Money` (decision 0040). `settle_intents(payer, payees, weights,
bill)` shares a bill among the accounts in proportion to `weights` with `Money::allocate`, so the
shares add up to the bill exactly, and turns each owed share into an ordinary transfer to the
payer; the ledger itself is unchanged. `run_bill_fixture(seed, rounds)` is a session of 2,000 bills
(a random payer, a random bill, random weights, settled in one step), and its hash is committed in
`fixtures/ledger-bills.hash` and checked natively, under WebAssembly and by
`lockstep-headless verify ledger-bills`. The tests check that the total never changes, that the
owed shares plus the payer's own share equal the bill, and that bad requests are refused.
