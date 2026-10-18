// SPDX-License-Identifier: Apache-2.0
use serde::{Deserialize, Serialize};
use std::fmt;

/// The largest currency exponent: ten to the 18th is the largest power of ten an `i64` holds.
pub const MAXIMUM_EXPONENT: u8 = 18;

/// A currency: a three letter code and the number of minor unit digits after the decimal point
/// (2 for cents, 0 for whole units, 3 for thousandths).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[serde(try_from = "CurrencyParts")]
pub struct Currency {
    code: [u8; 3],
    exponent: u8,
}

#[derive(Deserialize)]
struct CurrencyParts {
    code: [u8; 3],
    exponent: u8,
}

impl TryFrom<CurrencyParts> for Currency {
    type Error = MoneyError;
    fn try_from(parts: CurrencyParts) -> Result<Self, MoneyError> {
        Currency::from_bytes(parts.code, parts.exponent)
    }
}

impl Currency {
    /// `code` must be three uppercase ASCII letters and `exponent` at most `MAXIMUM_EXPONENT`.
    pub fn new(code: &str, exponent: u8) -> Result<Self, MoneyError> {
        let bytes: [u8; 3] = code
            .as_bytes()
            .try_into()
            .map_err(|_| MoneyError::InvalidCurrency)?;
        Currency::from_bytes(bytes, exponent)
    }

    fn from_bytes(code: [u8; 3], exponent: u8) -> Result<Self, MoneyError> {
        if !code.iter().all(|byte| byte.is_ascii_uppercase()) || exponent > MAXIMUM_EXPONENT {
            return Err(MoneyError::InvalidCurrency);
        }
        Ok(Currency { code, exponent })
    }

    pub fn code(&self) -> &str {
        std::str::from_utf8(&self.code).expect("the code is ASCII")
    }

    pub fn exponent(&self) -> u8 {
        self.exponent
    }

    /// Minor units in one major unit.
    pub fn minor_per_major(&self) -> i64 {
        10i64.pow(self.exponent as u32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoneyError {
    /// The code is not three uppercase letters, or the exponent is past `MAXIMUM_EXPONENT`.
    InvalidCurrency,
    /// The two amounts are in different currencies.
    CurrencyMismatch,
    /// The result does not fit in 64 bits of minor units.
    Overflow,
    /// A split into no parts, or allocation by weights that are all zero.
    NothingToShareWith,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            MoneyError::InvalidCurrency => "not a valid currency",
            MoneyError::CurrencyMismatch => "the amounts are in different currencies",
            MoneyError::Overflow => "the amount does not fit in 64 bits of minor units",
            MoneyError::NothingToShareWith => "there is no one to share the amount with",
        })
    }
}

impl std::error::Error for MoneyError {}

/// An amount of money: whole minor units of one currency. There is no float anywhere. Every
/// operation is checked and returns an error instead of wrapping or panicking, and sharing an
/// amount between parts never loses or creates a minor unit.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Money {
    minor: i64,
    currency: Currency,
}

impl Money {
    pub fn from_minor(minor: i64, currency: Currency) -> Self {
        Money { minor, currency }
    }

    /// `major` whole units, for example 12 dollars.
    pub fn from_major(major: i64, currency: Currency) -> Result<Self, MoneyError> {
        let minor = major
            .checked_mul(currency.minor_per_major())
            .ok_or(MoneyError::Overflow)?;
        Ok(Money { minor, currency })
    }

    pub fn zero(currency: Currency) -> Self {
        Money { minor: 0, currency }
    }

    pub fn minor(&self) -> i64 {
        self.minor
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn is_negative(&self) -> bool {
        self.minor < 0
    }

    fn with_minor(&self, minor: Option<i64>) -> Result<Money, MoneyError> {
        minor
            .map(|minor| Money {
                minor,
                currency: self.currency,
            })
            .ok_or(MoneyError::Overflow)
    }

    pub fn checked_add(&self, other: Money) -> Result<Money, MoneyError> {
        self.same_currency(other)?;
        self.with_minor(self.minor.checked_add(other.minor))
    }

    pub fn checked_sub(&self, other: Money) -> Result<Money, MoneyError> {
        self.same_currency(other)?;
        self.with_minor(self.minor.checked_sub(other.minor))
    }

    pub fn checked_neg(&self) -> Result<Money, MoneyError> {
        self.with_minor(self.minor.checked_neg())
    }

    /// Times a whole number.
    pub fn checked_mul(&self, factor: i64) -> Result<Money, MoneyError> {
        self.with_minor(self.minor.checked_mul(factor))
    }

    /// The sum of the amounts, which must all be in `currency`. An empty list sums to zero.
    pub fn checked_sum(
        amounts: impl IntoIterator<Item = Money>,
        currency: Currency,
    ) -> Result<Money, MoneyError> {
        amounts
            .into_iter()
            .try_fold(Money::zero(currency), |total, amount| {
                total.checked_add(amount)
            })
    }

    /// Shares the amount between `parts` equal parts. Whole minor units cannot always divide
    /// evenly, so the first parts get one more unit each; the parts always add up to the amount.
    pub fn split(&self, parts: u32) -> Result<Vec<Money>, MoneyError> {
        if parts == 0 {
            return Err(MoneyError::NothingToShareWith);
        }
        self.allocate(&vec![1; parts as usize])
    }

    /// Shares the amount in proportion to `weights` by the largest remainder method: each part
    /// gets its floor share and the leftover units go one each to the parts with the largest
    /// remainders, the earlier part winning a tie. The parts always add up to the amount, and a
    /// part with weight zero gets nothing.
    pub fn allocate(&self, weights: &[u64]) -> Result<Vec<Money>, MoneyError> {
        let sum: u128 = weights.iter().map(|weight| *weight as u128).sum();
        if sum == 0 {
            return Err(MoneyError::NothingToShareWith);
        }
        let total = self.minor.unsigned_abs() as u128;
        // total is at most 2^63 and a weight below 2^64, so the product fits in 128 bits.
        let mut shares: Vec<u128> = weights
            .iter()
            .map(|weight| total * *weight as u128 / sum)
            .collect();
        let mut leftover = total - shares.iter().sum::<u128>();
        let mut order: Vec<usize> = (0..weights.len())
            .filter(|index| weights[*index] != 0)
            .collect();
        order.sort_by_key(|index| {
            let remainder = total * weights[*index] as u128 % sum;
            (std::cmp::Reverse(remainder), *index)
        });
        for index in order {
            if leftover == 0 {
                break;
            }
            shares[index] += 1;
            leftover -= 1;
        }
        let negative = self.minor < 0;
        shares
            .into_iter()
            .map(|share| {
                let signed = if negative {
                    -(share as i128)
                } else {
                    share as i128
                };
                i64::try_from(signed)
                    .map(|minor| Money {
                        minor,
                        currency: self.currency,
                    })
                    .map_err(|_| MoneyError::Overflow)
            })
            .collect()
    }

    fn same_currency(&self, other: Money) -> Result<(), MoneyError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch)
        }
    }
}

/// Major units with the currency's digits, for example `-12.05 USD`; for display, never parsed.
impl fmt::Display for Money {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let per_major = self.currency.minor_per_major() as u64;
        let magnitude = self.minor.unsigned_abs();
        let sign = if self.minor < 0 { "-" } else { "" };
        let code = self.currency.code();
        if self.currency.exponent == 0 {
            return write!(formatter, "{sign}{magnitude} {code}");
        }
        write!(
            formatter,
            "{sign}{}.{:0width$} {code}",
            magnitude / per_major,
            magnitude % per_major,
            width = self.currency.exponent as usize
        )
    }
}
