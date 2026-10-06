use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Signed 16.16 fixed point: range about plus or minus 32,767, precision 1/65,536.
///
/// Addition, subtraction, multiplication, division and `from_ratio` wrap on overflow: a result too
/// large for the type keeps its low 32 bits, which is deterministic. Debug builds enable overflow
/// checks on the arithmetic in the rest of the project, but not here: every operation on this type is
/// explicitly wrapping, so debug and release always agree.
/// Dividing by zero panics.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug, Serialize, Deserialize,
)]
pub struct Fixed32(i32);

impl Fixed32 {
    pub const ONE: Fixed32 = Fixed32(1 << 16);
    pub const ZERO: Fixed32 = Fixed32(0);
    pub const HALF: Fixed32 = Fixed32(1 << 15);

    pub const fn from_int(value: i32) -> Self {
        Fixed32(value << 16)
    }

    pub const fn from_raw(raw: i32) -> Self {
        Fixed32(raw)
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    /// numerator / denominator, rounded to nearest with ties away from zero, so the result for a
    /// negative ratio is the mirror of the result for the same positive one. Use instead of float
    /// literals. Panics when the denominator is zero.
    pub const fn from_ratio(numerator: i32, denominator: i32) -> Self {
        let scaled = numerator as i64 * 65_536;
        let divisor = (denominator as i64).abs();
        let magnitude = (scaled.abs() + divisor / 2) / divisor;
        let negative = (scaled < 0) != (denominator < 0);
        Fixed32(if negative { -magnitude } else { magnitude } as i32)
    }

    pub const fn floor(self) -> i32 {
        self.0 >> 16
    }

    /// Rounds to the nearest whole number, with ties rounding up (3.5 gives 4, -3.5 gives -3). It
    /// is computed in 64 bits, so the values just under the top of the range round up to 32,768
    /// instead of wrapping to -32,768.
    pub const fn round(self) -> i32 {
        ((self.0 as i64 + (1 << 15)) >> 16) as i32
    }

    pub const fn abs(self) -> Self {
        Fixed32(self.0.wrapping_abs())
    }

    pub fn min(self, other: Self) -> Self {
        if self.0 < other.0 {
            self
        } else {
            other
        }
    }

    pub fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }

    pub fn clamp(self, low: Self, high: Self) -> Self {
        self.max(low).min(high)
    }

    /// Square root, rounded down to the nearest raw unit. Panics on a negative value.
    pub fn sqrt(self) -> Self {
        assert!(self.0 >= 0, "square root of a negative number");
        Fixed32(isqrt((self.0 as u64) << 16) as i32)
    }

    pub fn lerp(self, other: Self, t: Self) -> Self {
        self + (other - self) * t
    }

    /// Parses a decimal such as `"-12.375"`, rounded to the nearest raw unit with ties away from
    /// zero, so a value that fits 16.16 exactly parses exactly. Data files use this, never a float.
    pub fn parse_decimal(text: &str) -> Result<Self, ParseFixedError> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if (whole.is_empty() && fraction.is_empty()) || !all_digits(whole) || !all_digits(fraction)
        {
            return Err(ParseFixedError::NotADecimal);
        }
        let whole: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| ParseFixedError::OutOfRange)?
        };
        // Every 16.16 value needs at most 16 fraction digits; 18 keep the arithmetic exact in i64.
        if fraction.len() > 18 {
            return Err(ParseFixedError::TooManyDigits);
        }
        let mut numerator: i64 = 0;
        let mut denominator: i64 = 1;
        for byte in fraction.bytes() {
            numerator = numerator * 10 + (byte - b'0') as i64;
            denominator *= 10;
        }
        let scaled = numerator as i128 * 65_536;
        let mut fraction_raw = scaled / denominator as i128;
        if (scaled % denominator as i128) * 2 >= denominator as i128 {
            fraction_raw += 1;
        }
        let magnitude = whole as i128 * 65_536 + fraction_raw;
        let raw = if negative { -magnitude } else { magnitude };
        if raw < i32::MIN as i128 || raw > i32::MAX as i128 {
            return Err(ParseFixedError::OutOfRange);
        }
        Ok(Fixed32(raw as i32))
    }

    /// Display and the host boundary only; never feed the result back into the simulation.
    pub fn to_f32_for_display(self) -> f32 {
        self.0 as f32 / 65_536.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseFixedError {
    NotADecimal,
    OutOfRange,
    /// More than 18 digits after the point.
    TooManyDigits,
}

impl std::fmt::Display for ParseFixedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseFixedError::NotADecimal => write!(formatter, "not a decimal number"),
            ParseFixedError::OutOfRange => write!(formatter, "outside the 16.16 range"),
            ParseFixedError::TooManyDigits => {
                write!(formatter, "more than 18 digits after the point")
            }
        }
    }
}

impl std::error::Error for ParseFixedError {}

impl std::str::FromStr for Fixed32 {
    type Err = ParseFixedError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Fixed32::parse_decimal(text)
    }
}

impl Add for Fixed32 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Fixed32(self.0.wrapping_add(other.0))
    }
}

impl Sub for Fixed32 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Fixed32(self.0.wrapping_sub(other.0))
    }
}

impl Mul for Fixed32 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Fixed32(((self.0 as i64 * other.0 as i64) >> 16) as i32)
    }
}

impl Div for Fixed32 {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        Fixed32((((self.0 as i64) << 16) / other.0 as i64) as i32)
    }
}

impl Neg for Fixed32 {
    type Output = Self;
    fn neg(self) -> Self {
        Fixed32(self.0.wrapping_neg())
    }
}

impl AddAssign for Fixed32 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Fixed32 {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

/// Integer square root, floor, deterministic by construction.
pub const fn isqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut x = 1u64 << ((64 - value.leading_zeros()).div_ceil(2));
    loop {
        let y = (x + value / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}
