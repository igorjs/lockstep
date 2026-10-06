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
fn every_walk_arrives_and_never_moves_farther_than_its_step_or_away_from_the_target() {
    let mut streams = Streams::new(5);
    for round in 0..2_000 {
        let mut point = || {
            Vector2::new(
                Fixed32::from_raw(streams.range("walk", -2_000_000_000, 2_000_000_000)),
                Fixed32::from_raw(streams.range("walk", -2_000_000_000, 2_000_000_000)),
            )
        };
        let (start, target) = (point(), point());
        // Every tenth walk is a tiny step over a short distance, which is where truncation bites.
        let (start, target, step) = if round % 10 == 0 {
            let near = Vector2::new(
                Fixed32::from_raw(start.x.raw() % 300),
                Fixed32::from_raw(start.y.raw() % 300),
            );
            (near, Vector2::ZERO, Fixed32::from_raw(1 + round % 7))
        } else {
            (
                start,
                target,
                Fixed32::from_raw(streams.range("walk", 20_000_000, 200_000_000)),
            )
        };
        let (mut position, mut distance, mut arrived) = (start, start.distance(target), false);
        for _ in 0..20_000 {
            let (next, done) = position.toward(target, step);
            assert!(
                position.distance(next) <= step,
                "a move longer than the step"
            );
            assert!(next.distance(target) <= distance, "the distance grew");
            (position, distance, arrived) = (next, next.distance(target), done);
            if arrived {
                break;
            }
        }
        assert!(
            arrived,
            "a walk with a positive step must arrive (round {round})"
        );
        assert_eq!(position, target);
    }
}
