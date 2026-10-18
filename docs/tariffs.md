<!-- SPDX-License-Identifier: Apache-2.0 -->

# tariffs (example)

A consumer scenario that is not a game: accounts and transfers that charge a fee. It shows saves and
migrations.

- A transfer moves the amount and charges its sender a fee in basis points of the amount, rounded
  down, on top. The fee goes to `fees_collected_minor`, apart from every balance. A sender who cannot
  cover the amount and the fee is rejected with `InsufficientFunds` and nothing moves.
- Balances plus fees equal the opening money plus deposits.
- The first shape of the books (`BooksV1`) had no fee. A save written then is committed as bytes in
  `fixtures/legacy-v1.bin`. `migrations()` converts it to the current books with a fee of nothing, so
  the next transfer costs what it cost when the save was taken.

## Fixtures

- `fixtures/tariffs.hash`: the scripted session, 6,000 steps at a fee of 25 basis points.
- `fixtures/legacy-v1.bin`: the first shape of the books at step 1,500.
- `fixtures/legacy-v1.hash`: the hash after loading that save and running the script to step 6,000.
  It equals a session that charged no fee from the start, which a test also checks.

`cargo run -p tariffs --example generate_legacy_save -- examples/tariffs/fixtures` wrote the two legacy
files once. Tests read them and never write them.

## Headless

`lockstep-headless` records and replays `tariffs`, saves part way (`replay --until --save`) and
resumes (`resume`). See [lockstep-headless](lockstep-headless.md).
