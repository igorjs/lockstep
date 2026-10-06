//! Decision: shared crates use 16.16 fixed point, with integer square roots and integer-only
//! trigonometry from a committed table.
//! Alternative rejected: 32-bit floats everywhere, which are identical across platforms only by
//! discipline.
//! Would change if: a square root is more than one raw unit from the true value, or
//! sin squared plus cos squared drifts more than 8 raw units from one anywhere (the numbers to
//! beat), or any result differs between native and WebAssembly.

use lockstep_core::math::{atan2, cos, sin, Fixed32};
use lockstep_core::Streams;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn square_roots_are_exact_for_perfect_squares_and_within_one_raw_unit_elsewhere() {
    for root in 0..=180 {
        assert_eq!(
            Fixed32::from_int(root * root).sqrt(),
            Fixed32::from_int(root)
        );
    }
    let mut streams = Streams::new(21);
    for _ in 0..20_000 {
        let raw = streams.range("sqrt", 0, i32::MAX);
        let expected = ((raw as f64) * 65_536.0).sqrt().floor() as i64;
        assert!((Fixed32::from_raw(raw).sqrt().raw() as i64 - expected).abs() <= 1);
    }
}

#[test]
fn the_unit_circle_stays_on_the_circle_and_arctangent_returns_the_angle() {
    for turn in (0..=u16::MAX).step_by(3) {
        let (s, c) = (sin(turn).raw() as i64, cos(turn).raw() as i64);
        assert!(((s * s + c * c) >> 16).abs_diff(65_536) <= 8);
        let found = atan2(sin(turn), cos(turn));
        let difference = found.wrapping_sub(turn);
        assert!(difference.min(difference.wrapping_neg()) <= 4);
    }
}
