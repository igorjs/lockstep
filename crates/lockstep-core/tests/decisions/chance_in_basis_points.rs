//! Decision: a percentage crosses the roll boundary as basis points (`Chance`, 10,000 is 100
//! percent), and every roll compares integer draws.
//! Alternative rejected: float probabilities, where 0.1 plus 0.2 is not 0.3 and a chance computed
//! on one platform can round to a different roll on another.
//! Would change if: the three rolls ever hash differently natively and under WebAssembly (the
//! number to beat is zero differences over 300,000 rolls), or a design needs finer than 0.01
//! percent.

use lockstep_core::{hash_of, Chance, SmoothedState, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_three_rolls_hash_to_the_committed_value_natively_and_under_webassembly() {
    let mut streams = Streams::new(20_260_925);
    let mut state = SmoothedState::default();
    let mut results = Vec::with_capacity(300_000);
    for index in 0..100_000u32 {
        let chance = Chance::basis_points(index * 7_919 % 10_001);
        results.push(streams.roll("plain", chance));
        results.push(streams.roll_with_luck("luck", chance, (index % 7) as i8 - 3));
        results.push(streams.roll_smoothed("smoothed", chance, &mut state));
    }
    let committed = include_str!("../../fixtures/chance_rolls.hash").trim();
    assert_eq!(format!("{:016x}", hash_of(&results)), committed);
}
