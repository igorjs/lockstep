<!-- SPDX-License-Identifier: Apache-2.0 -->

# ledger (example)

The consumer scenario that is not a game: accounts and transfers.

- Balances are whole minor units (`i64`) in one currency. The money type arrives in milestone M16.
- A transfer moves exactly the amount or is rejected with a reason: `UnknownAccount`, `NotPositive`, `SameAccount`, `InsufficientFunds`, `Overflow`. Nothing panics and nothing changes on a rejection.
- Money is only created by deposits, so the sum of balances is the opening total plus deposits.

The fixture draws one random intent per step from its own stream set. Its hash is committed in
`fixtures/ledger.hash`.
