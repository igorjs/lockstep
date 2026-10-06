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

    /// Exact length rounded down to a raw unit. It works on the raw components in 64 bits, so
    /// it stays correct for every vector whose length fits in a `Fixed32`.
    pub fn length(self) -> Fixed32 {
        let x = self.x.raw() as i64;
        let y = self.y.raw() as i64;
        Fixed32::from_raw(isqrt((x * x + y * y) as u64) as i32)
    }

    pub fn distance(self, other: Self) -> Fixed32 {
        (self - other).length()
    }

    /// A vector of length one in the same direction (within rounding). Zero stays zero.
    pub fn normalised(self) -> Self {
        let length = self.length();
        if length == Fixed32::ZERO {
            return Vector2::ZERO;
        }
        Vector2 {
            x: self.x / length,
            y: self.y / length,
        }
    }

    pub fn dot(self, other: Self) -> Fixed32 {
        self.x * other.x + self.y * other.y
    }

    /// Move toward `target` by at most `step`; returns the new position and whether it arrived.
    /// Never overshoots: the distance to the target never grows, and it arrives exactly.
    pub fn toward(self, target: Self, step: Fixed32) -> (Self, bool) {
        let delta = target - self;
        let remaining = delta.length();
        if remaining <= step {
            return (target, true);
        }
        // Scale each component by step / remaining in 64 bits. Division truncates toward
        // zero, so the move is never longer than `step`.
        let scale = |component: Fixed32| -> Fixed32 {
            Fixed32::from_raw(
                ((component.raw() as i64 * step.raw() as i64) / remaining.raw() as i64) as i32,
            )
        };
        (
            Vector2 {
                x: self.x + scale(delta.x),
                y: self.y + scale(delta.y),
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
