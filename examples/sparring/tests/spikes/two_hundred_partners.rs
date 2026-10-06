// SPDX-License-Identifier: Apache-2.0
//! Two hundred partners spar for 10,000 steps on one floor: no two bodies ever share a cell, and
//! no health ever goes below zero.

use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use sparring::{coach, crowd, runner, Event};
use std::collections::BTreeSet;

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a minute unoptimised; `just test` runs it with --release"
)]
fn two_hundred_partners_for_ten_thousand_steps_never_share_a_cell_or_go_below_zero_health() {
    let seed = 11;
    let mut runner = runner(crowd(100), seed);
    let partners = runner.simulation().partners();
    assert_eq!(partners.len(), 200);
    let mut script = Streams::new(seed);
    let (mut landed, mut yields) = (0u32, 0u32);
    for step in 0..10_000 {
        let intents = coach(runner.simulation(), &mut script, step);
        for event in runner.step_once(&intents).events {
            match event {
                Event::Hurt { .. } => landed += 1,
                Event::Yielded { .. } => yields += 1,
                _ => {}
            }
        }
        let simulation = runner.simulation();
        let mut cells = BTreeSet::new();
        for who in &partners {
            let cell = simulation
                .cell_of(*who)
                .expect("every partner is on the floor");
            assert!(cells.insert(cell), "step {step}: two bodies on {cell:?}");
            assert_eq!(simulation.world().occupancy.at(cell), Some(*who));
            assert!(simulation.health_of(*who).unwrap() >= Fixed32::ZERO);
        }
    }
    assert!(landed > 10_000, "the partners fought: {landed} hits");
    assert!(yields > 100, "and bouts were won: {yields}");
}
