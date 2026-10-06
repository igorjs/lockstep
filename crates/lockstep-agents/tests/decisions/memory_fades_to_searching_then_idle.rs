// SPDX-License-Identifier: Apache-2.0
//! Decision: an agent hunts its memory, whose confidence fades linearly with its age in exact
//! game minutes. Alert turns to Searching once the target has been out of sight for the rules'
//! minutes, and anything turns Idle when the memory is forgotten. Each change is timed, so a
//! crowd ripples instead of flipping at once.
//! Alternative rejected: tracking the target itself, which makes every agent omniscient; and
//! confidence lowered by a fraction each step, which drifts with the step rate.
//! Would change if: with 2 minutes to lose sight and 10 to forget, an agent turns Searching at
//! any time but 2 minutes, or Idle at any time but 10.

use crate::common::{minutes, Floor};
use lockstep_agents::{Alertness, MindEvent, Stimulus};
use lockstep_core::math::Fixed32;
use lockstep_core::Column;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn out_of_sight_for_two_minutes_searches_and_forgotten_at_ten_goes_idle() {
    for steps_per_minute in [1_800, 3_600] {
        let floor = Floor::new(40, 20, &[(5, 5), (15, 5)]);
        let (agent, target) = (floor.bodies[0], floor.bodies[1]);
        let mut minds = floor.minds();
        let leashes = Column::new();
        let saw = [(
            agent,
            Stimulus::Saw {
                target,
                at: floor.cell(15, 5),
            },
        )];
        floor.think(&mut minds, &leashes, &saw, Fixed32::ZERO, 8);
        assert_eq!(minds.get(agent).unwrap().alertness, Alertness::Alert);
        // A step's exact share of a minute; the shares of one minute add up to it exactly.
        let per_step = minutes(1).raw() / steps_per_minute;
        let mut changes = Vec::new();
        for step in 1..=(11 * steps_per_minute) {
            let raw = per_step
                + i32::from((step - 1) % steps_per_minute < minutes(1).raw() % steps_per_minute);
            for event in floor.think(&mut minds, &leashes, &[], Fixed32::from_raw(raw), 8) {
                if let MindEvent::Changed { to, .. } = event {
                    changes.push((to, step));
                }
            }
        }
        assert_eq!(
            changes,
            vec![
                (Alertness::Searching, 2 * steps_per_minute),
                (Alertness::Idle, 10 * steps_per_minute),
            ],
            "at {steps_per_minute} steps a minute"
        );
    }
}
