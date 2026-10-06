use super::fixed::{isqrt, Fixed32};
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

    /// The exact length in raw units, rounded down, in 64 bits so it never wraps: even the longest
    /// vector, from the smallest to the largest corner, fits.
    fn length_raw(self) -> u64 {
        let x = self.x.raw() as i64;
        let y = self.y.raw() as i64;
        // Each square is at most 2^62, so the sum fits in an unsigned 64-bit integer.
        isqrt((x * x) as u64 + (y * y) as u64)
    }

    /// The length, rounded down to a raw unit. A length too long for a `Fixed32` (over 32,767
    /// units) saturates at the largest value instead of wrapping to a negative one.
    pub fn length(self) -> Fixed32 {
        Fixed32::from_raw(self.length_raw().min(i32::MAX as u64) as i32)
    }

    pub fn distance(self, other: Self) -> Fixed32 {
        (self - other).length()
    }

    /// A vector of length one in the same direction (within rounding). Zero stays zero.
    pub fn normalised(self) -> Self {
        let length = self.length_raw() as i64;
        if length == 0 {
            return Vector2::ZERO;
        }
        // A component is never longer than the vector, so the quotient fits in a `Fixed32`.
        Vector2 {
            x: Fixed32::from_raw((((self.x.raw() as i64) << 16) / length) as i32),
            y: Fixed32::from_raw((((self.y.raw() as i64) << 16) / length) as i32),
        }
    }

    pub fn dot(self, other: Self) -> Fixed32 {
        self.x * other.x + self.y * other.y
    }

    /// Move toward `target` by at most `step`; returns the new position and whether it arrived.
    /// A negative step counts as zero. Never overshoots: a move is never longer than `step`, the
    /// distance to the target never grows, and arrival is exact. A positive step always makes
    /// progress, even one raw unit along a diagonal.
    pub fn toward(self, target: Self, step: Fixed32) -> (Self, bool) {
        let step = step.raw().max(0) as i64;
        let delta = target - self;
        let remaining = delta.length_raw() as i64;
        if remaining <= step {
            return (target, true);
        }
        // Scale each component by step / remaining in 64 bits. Division truncates toward zero, so
        // the move is never longer than `step`.
        let scale = |component: Fixed32| (component.raw() as i64 * step / remaining) as i32;
        let (mut move_x, mut move_y) = (scale(delta.x), scale(delta.y));
        if step > 0 && move_x == 0 && move_y == 0 {
            // Both parts truncated away (a tiny step along a diagonal). Take one raw unit along
            // the longer axis: that still shortens the distance and is no longer than the step.
            if delta.x.raw().abs() >= delta.y.raw().abs() {
                move_x = delta.x.raw().signum();
            } else {
                move_y = delta.y.raw().signum();
            }
        }
        (
            Vector2 {
                x: self.x + Fixed32::from_raw(move_x),
                y: self.y + Fixed32::from_raw(move_y),
            },
            false,
        )
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
