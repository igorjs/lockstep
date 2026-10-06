//! Decision: `Vector2::toward` truncates its step toward zero, so it never moves farther than
//! asked, and it arrives exactly instead of oscillating around the target.
//! Alternative rejected: normalising the direction and multiplying by the step, whose rounding can
//! move slightly too far and overshoot.
//! Would change if: any step increases the distance to the target (the number to beat is zero
//! increases over 2,000 random walks) or a walk fails to arrive.

use lockstep_core::math::{Fixed32, Vector2};
use lockstep_core::Streams;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn every_walk_arrives_and_never_gets_farther_away() {
    let mut streams = Streams::new(5);
    for _ in 0..2_000 {
        let mut point = || {
            Vector2::new(
                Fixed32::from_raw(streams.range("walk", -300_000, 300_000)),
                Fixed32::from_raw(streams.range("walk", -300_000, 300_000)),
            )
        };
        let (start, target) = (point(), point());
        let step = Fixed32::from_raw(streams.range("walk", 1_000, 90_000));
        let (mut position, mut distance, mut arrived) = (start, start.distance(target), false);
        for _ in 0..4_000 {
            let (next, done) = position.toward(target, step);
            assert!(next.distance(target) <= distance);
            (position, distance, arrived) = (next, next.distance(target), done);
            if arrived {
                break;
            }
        }
        assert!(arrived, "a walk with a positive step must arrive");
        assert_eq!(position, target);
    }
}
