use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Signed 16.16 fixed point: range about plus or minus 32,767, precision 1/65,536.
///
/// Addition, subtraction and multiplication wrap on overflow, which is deterministic. Debug
/// builds enable overflow checks on the arithmetic in the rest of the project, but not here:
/// every operation on this type is explicitly wrapping, so debug and release always agree.
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

    /// numerator / denominator, rounded to nearest. Use instead of float literals.
    pub const fn from_ratio(numerator: i32, denominator: i32) -> Self {
        Fixed32(
            ((numerator as i64 * 65_536 + (denominator as i64) / 2) / denominator as i64) as i32,
        )
    }

    pub const fn floor(self) -> i32 {
        self.0 >> 16
    }

    pub const fn round(self) -> i32 {
        self.0.wrapping_add(1 << 15) >> 16
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

    /// Display and the host boundary only; never feed the result back into the simulation.
    pub fn to_f32_for_display(self) -> f32 {
        self.0 as f32 / 65_536.0
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
