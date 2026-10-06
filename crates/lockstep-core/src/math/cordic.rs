//! Integer CORDIC, run at compile time, for the sine and arctangent tables.
//!
//! No floats are involved anywhere: the constants below are plain integers, so the tables are the
//! same on every platform. The committed fixtures (`fixtures/quarter_sine.bin` and
//! `fixtures/atan_octant.bin`) pin them, and a test regenerates and compares.

/// 32 fractional bits.
const SCALE_BITS: u32 = 32;
const ITERATIONS: usize = 33;
/// atan(2^-k) for k = 0.., in radians with 32 fractional bits.
const ATAN: [i64; ITERATIONS] = [
    3373259426, 1991351318, 1052175346, 534100635, 268086748, 134174063, 67103403, 33553749,
    16777131, 8388597, 4194303, 2097152, 1048576, 524288, 262144, 131072, 65536, 32768, 16384,
    8192, 4096, 2048, 1024, 512, 256, 128, 64, 32, 16, 8, 4, 2, 1,
];
/// The CORDIC gain correction, 0.6072529350088812 with 32 fractional bits.
const GAIN: i64 = 2608131496;
/// Pi divided by two, with 32 fractional bits.
const HALF_PI: i64 = 6746518852;
/// Two times pi, with 32 fractional bits.
const TWO_PI: i64 = 26986075409;

/// Entries in the quarter-turn sine table: one per four turn units, plus the end point.
pub const QUARTER_SINE_ENTRIES: usize = 4_097;
/// Entries in the arctangent table for tangents from 0 to 1 in steps of 1/1,024, plus the end point.
pub const ATAN_OCTANT_ENTRIES: usize = 1_025;

/// Sine of a quarter turn in 4,096 equal steps, in 16.16 fixed point raw units.
pub const fn generate_quarter_sine() -> [i32; QUARTER_SINE_ENTRIES] {
    let mut table = [0i32; QUARTER_SINE_ENTRIES];
    let mut index = 0;
    while index < QUARTER_SINE_ENTRIES {
        let mut z = index as i64 * HALF_PI / 4_096;
        let mut x = GAIN;
        let mut y = 0i64;
        let mut step = 0;
        while step < ITERATIONS {
            let (shifted_x, shifted_y) = (x >> step, y >> step);
            if z >= 0 {
                x -= shifted_y;
                y += shifted_x;
                z -= ATAN[step];
            } else {
                x += shifted_y;
                y -= shifted_x;
                z += ATAN[step];
            }
            step += 1;
        }
        // 32 fractional bits to 16, rounded to nearest.
        table[index] = ((y + (1 << (SCALE_BITS - 17))) >> (SCALE_BITS - 16)) as i32;
        index += 1;
    }
    table
}

/// Arctangent of `i / 1,024`, in whole turns scaled by 2^32 (so 0.125 turns, the angle of a
/// tangent of one, is 2^29). Vectoring-mode CORDIC.
pub const fn generate_atan_octant() -> [u32; ATAN_OCTANT_ENTRIES] {
    let mut table = [0u32; ATAN_OCTANT_ENTRIES];
    let mut index = 0;
    while index < ATAN_OCTANT_ENTRIES {
        let mut x = 1i64 << SCALE_BITS;
        let mut y = (index as i64) << (SCALE_BITS - 10);
        let mut z = 0i64;
        let mut step = 0;
        while step < ITERATIONS {
            let (shifted_x, shifted_y) = (x >> step, y >> step);
            if y >= 0 {
                x += shifted_y;
                y -= shifted_x;
                z += ATAN[step];
            } else {
                x -= shifted_y;
                y += shifted_x;
                z -= ATAN[step];
            }
            step += 1;
        }
        // Radians with 32 fractional bits to turns with 32 fractional bits, rounded to nearest.
        table[index] =
            ((((z as i128) << SCALE_BITS) + (TWO_PI / 2) as i128) / TWO_PI as i128) as u32;
        index += 1;
    }
    table
}
