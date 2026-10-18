// SPDX-License-Identifier: Apache-2.0
//! Decision: a wave admits at most the headroom the director leaves, and arrivals the budget
//! turns away wait for the next step, so none is dropped and the load never passes the budget.
//! Alternative rejected: dropping the excess, which loses arrivals; and admitting a whole pulse
//! at once, which breaks the budget the director exists to keep.
//! Would change if: the load, after each step, ever passes the budget, or the admitted arrivals
//! of a finished wave differ from its total.

use lockstep_agents::{Director, WaveRules, Waves};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_pulse_larger_than_the_budget_is_admitted_over_several_steps() {
    let rules = WaveRules {
        pulse_size: 50,
        interval_minutes: 10,
        start_after_minutes: 0,
    };
    let director = Director { alert_budget: 8 };
    let mut waves = Waves::new(200);
    let mut load = 0u32;
    let mut peak = 0;
    for _ in 0..2_000 {
        // Three leave every step.
        load = load.saturating_sub(3);
        load += waves.advance(
            &rules,
            Fixed32::from_ratio(1, 2),
            director.headroom(load),
            &mut Vec::new(),
        );
        peak = peak.max(load);
        assert!(load <= u32::from(director.alert_budget));
    }
    assert!(waves.is_finished());
    assert_eq!(waves.admitted(), 200);
    assert_eq!(peak, 8);
}
