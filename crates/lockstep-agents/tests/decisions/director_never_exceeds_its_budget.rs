// SPDX-License-Identifier: Apache-2.0
//! Decision: the director grants Alert on one target to at most its budget of agents, nearest
//! the target first, then those already Alert, then by handle; the rest stay Curious, so a crowd
//! does not converge all at once. A stimulus outside an agent's leash is ignored.
//! Alternative rejected: first come first served, which lets the agents that happened to look
//! first hold the tokens while nearer ones wait.
//! Would change if: twenty agents seeing one target with a budget of eight ever leave more than
//! eight Alert, or leave anyone but the eight nearest Alert; or a leashed agent reacts to a
//! noise outside its leash.

use crate::common::{metres, Floor};
use lockstep_agents::{Alertness, Leash, MindEvent, Stimulus};
use lockstep_core::math::Fixed32;
use lockstep_core::Column;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn twenty_agents_seeing_one_target_leave_the_eight_nearest_alert() {
    // The target at (0, 10); agents at x = 1 to 20 along row 10, nearest first in x.
    let mut cells = vec![(0, 10)];
    cells.extend((1..=20).map(|x| (x, 10)));
    let floor = Floor::new(40, 20, &cells);
    let target = floor.bodies[0];
    let agents = &floor.bodies[1..];
    let mut minds = floor.minds();
    minds.unset(target);
    let leashes = Column::new();
    let at = floor.cell(0, 10);
    for step in 0..50 {
        // Every agent sees the target on even steps; on odd steps a few far ones see it.
        let stimuli: Vec<_> = agents
            .iter()
            .enumerate()
            .filter(|(index, _)| step % 2 == 0 || index % 5 == 4)
            .map(|(_, agent)| (*agent, Stimulus::Saw { target, at }))
            .collect();
        let events = floor.think(
            &mut minds,
            &leashes,
            &stimuli,
            Fixed32::from_ratio(1, 30),
            8,
        );
        let alert: Vec<_> = agents
            .iter()
            .filter(|agent| minds.get(**agent).unwrap().alertness == Alertness::Alert)
            .copied()
            .collect();
        assert!(alert.len() <= 8, "step {step}: {} alert", alert.len());
        if step == 0 {
            assert_eq!(alert, agents[..8].to_vec(), "the eight nearest");
            let held = events
                .iter()
                .filter(|event| matches!(event, MindEvent::Held { .. }))
                .count();
            assert_eq!(held, 12);
        }
    }
}

#[test]
fn a_leashed_agent_ignores_a_noise_beyond_its_leash() {
    let floor = Floor::new(60, 20, &[(5, 10)]);
    let agent = floor.bodies[0];
    let mut minds = floor.minds();
    let mut leashes = Column::new();
    leashes.set(
        agent,
        Leash {
            home: floor.cell(5, 10),
            radius_metres: metres(10),
        },
    );
    let far = [(
        agent,
        Stimulus::Heard {
            at: floor.cell(35, 10),
            source: None,
        },
    )];
    floor.think(&mut minds, &leashes, &far, Fixed32::ZERO, 8);
    assert_eq!(
        minds.get(agent).unwrap().alertness,
        Alertness::Idle,
        "15 metres off"
    );
    let near = [(
        agent,
        Stimulus::Heard {
            at: floor.cell(25, 10),
            source: None,
        },
    )];
    floor.think(&mut minds, &leashes, &near, Fixed32::ZERO, 8);
    assert_eq!(
        minds.get(agent).unwrap().alertness,
        Alertness::Curious,
        "10 metres off"
    );
}
