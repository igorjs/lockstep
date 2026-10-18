<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0040 Money is whole minor units, shared by largest remainder (decided)

Status: decided and implemented in milestone M16.

## Decision

- `Money` is an `i64` count of minor units plus a `Currency` (three capital letters and an
  exponent from 0 to 18, the digits after the decimal point). There is no float anywhere.
- Every operation is checked and returns `MoneyError` (`Overflow`, `CurrencyMismatch`,
  `InvalidCurrency`, `NothingToShareWith`): `checked_add`, `checked_sub`, `checked_neg`,
  `checked_mul` by a whole number, `checked_sum`, `from_major`. Nothing wraps or panics.
- `allocate(weights)` and `split(parts)` share an amount by the largest remainder method: each
  part gets its floor share, and the leftover units go one each to the parts with the largest
  remainders, the earlier part winning a tie. The parts always add up to the amount; a part with
  weight zero gets nothing; the share of a negative amount mirrors the share of a positive one.
  The arithmetic is in 128 bits, so even `i64::MIN` shared by weights near `u64::MAX` is exact.
- A decoded `Currency` is checked like a new one, so a damaged save cannot carry an exponent that
  overflows a power of ten.

## Alternatives rejected

- A float or `Fixed32` amount: each part rounds on its own, so a third of 1.00 three times is
  0.99.
- Rounding the last part to absorb the difference: it hides who paid the extra unit and lets one
  part drift by many units over many shares.
- Mixing currencies with an exchange rate inside `Money`: a rate is data the caller owns.

## Would change if

`money_shares_never_lose_a_unit.rs` fails: a share of 20,000 random amounts and weight lists must
add up exactly and stay within one unit of the exact proportion.
