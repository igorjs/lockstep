//! Deterministic math: 16.16 fixed point, vectors, and angles as whole turns.
//!
//! Everything here uses integers only. The lint in `scripts/lint-determinism.sh` points here for
//! anything a simulation would otherwise do with floats. Floats stay allowed for rates and display,
//! but shared crates prefer `Fixed32`.

mod angle;
mod cordic;
mod fixed;
mod vector;

pub use angle::{atan2, cos, sin, unit, unit_circle_table, Turn};
pub use cordic::{
    generate_atan_octant, generate_quarter_sine, ATAN_OCTANT_ENTRIES, QUARTER_SINE_ENTRIES,
};
pub use fixed::{isqrt, Fixed32};
pub use vector::Vector2;
