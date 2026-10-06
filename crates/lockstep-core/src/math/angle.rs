//! Angles are whole turns: `Turn = u16`, 65,536 per full turn, 0 east, 16,384 north. No radians.

use super::cordic::{
    generate_atan_octant, generate_quarter_sine, ATAN_OCTANT_ENTRIES, QUARTER_SINE_ENTRIES,
};
use super::fixed::Fixed32;
use super::vector::Vector2;

pub type Turn = u16;

static QUARTER_SINE: [i32; QUARTER_SINE_ENTRIES] = generate_quarter_sine();
static ATAN_OCTANT: [u32; ATAN_OCTANT_ENTRIES] = generate_atan_octant();

const QUARTER: u32 = 16_384;

/// Sine of an angle inside the first quarter turn (0 to 16,384 inclusive), linearly interpolated
/// between table entries four turn units apart.
fn first_quarter(offset: u32) -> i32 {
    let index = (offset >> 2) as usize;
    let fraction = (offset & 3) as i32;
    let low = QUARTER_SINE[index];
    let high = QUARTER_SINE[(index + 1).min(QUARTER_SINE_ENTRIES - 1)];
    low + (((high - low) * fraction + 2) >> 2)
}

/// Sine: quadrant fold and table lookup.
pub fn sin(turn: Turn) -> Fixed32 {
    let quadrant = turn >> 14;
    let offset = (turn & 0x3fff) as u32;
    let raw = match quadrant {
        0 => first_quarter(offset),
        1 => first_quarter(QUARTER - offset),
        2 => -first_quarter(offset),
        _ => -first_quarter(QUARTER - offset),
    };
    Fixed32::from_raw(raw)
}

pub fn cos(turn: Turn) -> Fixed32 {
    sin(turn.wrapping_add(QUARTER as u16))
}

/// The unit vector for an angle: cone directions, isotropy tests.
pub fn unit(turn: Turn) -> Vector2 {
    Vector2::new(cos(turn), sin(turn))
}

/// The angle of the point (x, y) as a whole-turn angle, 0 east, counter-clockwise. The origin
/// gives 0. Octant fold plus a 1,025-entry table with interpolation: the error is within one turn
/// unit, well inside 0.01 degrees.
pub fn atan2(y: Fixed32, x: Fixed32) -> Turn {
    let (x_raw, y_raw) = (x.raw() as i64, y.raw() as i64);
    if x_raw == 0 && y_raw == 0 {
        return 0;
    }
    let (absolute_x, absolute_y) = (x_raw.abs(), y_raw.abs());
    let steep = absolute_y > absolute_x;
    let (low, high) = if steep {
        (absolute_x, absolute_y)
    } else {
        (absolute_y, absolute_x)
    };
    // The tangent in 0..=1 with 20 fractional bits.
    let tangent = (low << 20) / high;
    let index = (tangent >> 10) as usize;
    let fraction = tangent & 1023;
    let base = ATAN_OCTANT[index] as i64;
    let next = ATAN_OCTANT[(index + 1).min(ATAN_OCTANT_ENTRIES - 1)] as i64;
    let interpolated = base + (((next - base) * fraction) >> 10);
    // Turns scaled by 2^32 to whole turn units (2^16 per turn), rounded to nearest.
    let in_octant = ((interpolated + (1 << 15)) >> 16) as u32;
    let first_quadrant = if steep {
        QUARTER - in_octant
    } else {
        in_octant
    };
    let turn = match (x_raw >= 0, y_raw >= 0) {
        (true, true) => first_quadrant,
        (false, true) => 32_768 - first_quadrant,
        (false, false) => 32_768 + first_quadrant,
        (true, false) => 65_536 - first_quadrant,
    };
    turn as u16
}

/// The integer point `index` of `count` evenly spaced points on a circle of `radius` cells, as
/// (x, y), rounded to the nearest cell. Starts at east and goes counter-clockwise.
pub fn unit_circle_table(index: u32, count: u32, radius: i32) -> (i32, i32) {
    let turn = ((index as u64 * 65_536) / count.max(1) as u64) as u16;
    let scale = |ratio: Fixed32| ((ratio.raw() as i64 * radius as i64 + (1 << 15)) >> 16) as i32;
    (scale(cos(turn)), scale(sin(turn)))
}
