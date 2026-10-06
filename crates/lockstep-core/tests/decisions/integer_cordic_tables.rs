// SPDX-License-Identifier: Apache-2.0
//! Decision: the sine and arctangent tables are generated at compile time by integer CORDIC and
//! committed as fixtures, and a test regenerates them and compares.
//! Alternative rejected: filling the tables from the standard library's float sine, whose last
//! bits may differ between platforms and library versions.
//! Would change if: the regenerated table differs from the committed one on any platform (the
//! number to beat is zero differing entries), or the committed sine is more than 2 raw units from
//! the true curve.

use lockstep_core::math::{
    generate_atan_octant, generate_quarter_sine, sin, ATAN_OCTANT_ENTRIES, QUARTER_SINE_ENTRIES,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_tables_have_the_documented_size_and_end_points() {
    let sine = generate_quarter_sine();
    assert_eq!(sine.len(), QUARTER_SINE_ENTRIES);
    assert_eq!((sine[0], sine[4_096]), (0, 65_536));
    let atan = generate_atan_octant();
    assert_eq!(atan.len(), ATAN_OCTANT_ENTRIES);
    assert_eq!(
        (atan[0], atan[1_024]),
        (0, 1 << 29),
        "an eighth of a turn, scaled by 2^32"
    );
}

#[test]
fn the_tables_equal_the_committed_fixtures_byte_for_byte() {
    let sine: Vec<u8> = generate_quarter_sine()
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect();
    assert_eq!(
        sine.as_slice(),
        include_bytes!("../../fixtures/quarter_sine.bin").as_slice()
    );
    let atan: Vec<u8> = generate_atan_octant()
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect();
    assert_eq!(
        atan.as_slice(),
        include_bytes!("../../fixtures/atan_octant.bin").as_slice()
    );
}

#[test]
fn the_committed_sine_is_within_two_raw_units_of_the_true_curve() {
    for turn in (0..16_384u32).step_by(5) {
        let radians = turn as f64 / 65_536.0 * std::f64::consts::TAU;
        let expected = (radians.sin() * 65_536.0).round() as i64;
        assert!((sin(turn as u16).raw() as i64 - expected).abs() <= 2);
    }
}
