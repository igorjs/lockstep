// SPDX-License-Identifier: Apache-2.0
use super::fixed::Fixed32;
use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct Vector2 {
    pub x: Fixed32,
    pub y: Fixed32,
}

impl Vector2 {
    pub const ZERO: Vector2 = Vector2 {
        x: Fixed32::ZERO,
        y: Fixed32::ZERO,
    };

    pub const fn new(x: Fixed32, y: Fixed32) -> Self {
        Vector2 { x, y }
    }

    /// The exact length in raw units, rounded down. It works on the raw components in 128 bits, so
    /// it never wraps, even for the longest vector or the difference of two far-apart points.
    fn length_raw(self) -> u128 {
        let x = self.x.raw() as i128;
        let y = self.y.raw() as i128;
        isqrt_u128((x * x + y * y) as u128)
    }

    /// The exact distance to another point in raw units, rounded down. The difference is taken in
    /// 128 bits, so two points more than 32,767 units apart on one axis are measured correctly.
    fn distance_raw(self, other: Self) -> u128 {
        let dx = self.x.raw() as i128 - other.x.raw() as i128;
        let dy = self.y.raw() as i128 - other.y.raw() as i128;
        isqrt_u128((dx * dx + dy * dy) as u128)
    }

    /// The length, rounded down to a raw unit. A length too long for a `Fixed32` (over 32,767
    /// units) saturates at the largest value instead of wrapping to a negative one.
    pub fn length(self) -> Fixed32 {
        Fixed32::from_raw(self.length_raw().min(i32::MAX as u128) as i32)
    }

    /// The distance to another point, with the same saturation as `length`.
    pub fn distance(self, other: Self) -> Fixed32 {
        Fixed32::from_raw(self.distance_raw(other).min(i32::MAX as u128) as i32)
    }

    /// A vector of length one in the same direction (within rounding). Zero stays zero.
    pub fn normalised(self) -> Self {
        let length = self.length_raw() as i128;
        if length == 0 {
            return Vector2::ZERO;
        }
        // A component is never longer than the vector, so the quotient fits in a `Fixed32`.
        Vector2 {
            x: Fixed32::from_raw((((self.x.raw() as i128) << 16) / length) as i32),
            y: Fixed32::from_raw((((self.y.raw() as i128) << 16) / length) as i32),
        }
    }

    pub fn dot(self, other: Self) -> Fixed32 {
        self.x * other.x + self.y * other.y
    }

    /// Move toward `target` by at most `step`; returns the new position and whether it arrived.
    /// A negative step counts as zero. Never overshoots: a move is never longer than `step`, the
    /// distance to the target never grows, and arrival is exact. A positive step always makes
    /// progress, even one raw unit along a diagonal. The distance and the direction are computed
    /// in 128 bits, so it works across the whole range, including two points on opposite sides of
    /// the range more than 32,767 units apart.
    pub fn toward(self, target: Self, step: Fixed32) -> (Self, bool) {
        let step = step.raw().max(0) as i128;
        let delta_x = target.x.raw() as i128 - self.x.raw() as i128;
        let delta_y = target.y.raw() as i128 - self.y.raw() as i128;
        let remaining = isqrt_u128((delta_x * delta_x + delta_y * delta_y) as u128) as i128;
        if remaining <= step {
            return (target, true);
        }
        // Scale each component by step / remaining. Division truncates toward zero, so the move is
        // never longer than `step`, and each part fits in an `i32` because it is at most `step`.
        let mut move_x = (delta_x * step / remaining) as i32;
        let mut move_y = (delta_y * step / remaining) as i32;
        if step > 0 && move_x == 0 && move_y == 0 {
            // Both parts truncated away (a tiny step along a diagonal). Take one raw unit along the
            // longer axis: that still shortens the distance and is no longer than the step.
            if delta_x.abs() >= delta_y.abs() {
                move_x = delta_x.signum() as i32;
            } else {
                move_y = delta_y.signum() as i32;
            }
        }
        // The new position lies between the two points, so adding the move cannot leave the range.
        (
            Vector2 {
                x: Fixed32::from_raw(self.x.raw() + move_x),
                y: Fixed32::from_raw(self.y.raw() + move_y),
            },
            false,
        )
    }
}

/// Integer square root of a 128-bit number, rounded down.
fn isqrt_u128(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut x = 1u128 << (128 - value.leading_zeros()).div_ceil(2);
    loop {
        let y = (x + value / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

impl Add for Vector2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Vector2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vector2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Vector2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Neg for Vector2 {
    type Output = Self;
    fn neg(self) -> Self {
        Vector2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Mul<Fixed32> for Vector2 {
    type Output = Self;
    fn mul(self, scale: Fixed32) -> Self {
        Vector2 {
            x: self.x * scale,
            y: self.y * scale,
        }
    }
}
