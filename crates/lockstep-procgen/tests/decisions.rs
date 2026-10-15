// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each states the decision, the alternative that was rejected, and
//! the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

use lockstep_procgen::{chunk_seed, Templates};
use std::collections::BTreeSet;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const TEMPLATES: &str = r##"{
  "side": 2,
  "templates": [
    { "name": "meadow", "rows": ["..", ".."] },
    { "name": "rocks", "rows": ["#.", ".#"] },
    { "name": "marsh", "rows": ["~~", "~~"] }
  ],
  "pools": [ { "name": "wild", "entries": [
    { "template": "meadow", "weight": 4 }, { "template": "rocks", "weight": 3 },
    { "template": "marsh", "weight": 1 } ] } ]
}"##;

/// Decision: every chunk draws from a stream of its own, seeded by the world seed and the
/// chunk's place alone. A chunk rolls the same whatever the size of the layout, so a world can
/// grow without the chunks already seen changing.
/// Alternative rejected: one stream walked across all chunks in order, where a bigger layout
/// or a reservation that skips a draw shifts every chunk after it.
/// Would change if: a chunk differs between a 3 by 3 and a 9 by 9 layout of one seed, or moves
/// when a reservation is added elsewhere.
#[test]
fn a_chunk_rolls_the_same_in_a_bigger_layout() {
    let templates = Templates::from_json(TEMPLATES).unwrap();
    let wild = templates.pool_id("wild").unwrap();
    let small = templates.roll(99, 3, 3, wild, &[]).unwrap();
    let big = templates.roll(99, 9, 9, wild, &[]).unwrap();
    for y in 0..3 {
        for x in 0..3 {
            assert_eq!(small.template_at(x, y), big.template_at(x, y));
        }
    }
}

/// Decision, the gate: seeds never collide. For one world seed every chunk place has its own
/// seed, because the mix is a bijection of 64 bits; across many world seeds no two chunks share
/// one either.
/// Alternative rejected: adding the coordinates to the world seed, where chunk (1, 0) of seed 0
/// is chunk (0, 1) of seed 1 and the two rolls repeat each other.
/// Would change if: two of the 200 world seeds times 128 by 128 chunk places share a seed.
#[test]
fn chunk_seeds_never_collide() {
    let mut seen = BTreeSet::new();
    for world_seed in 0..200u64 {
        for y in 0..128 {
            for x in 0..128 {
                assert!(
                    seen.insert(chunk_seed(world_seed, x, y)),
                    "collision at seed {world_seed} chunk ({x}, {y})"
                );
            }
        }
    }
    assert_eq!(seen.len(), 200 * 128 * 128);
}
