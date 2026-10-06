// SPDX-License-Identifier: Apache-2.0
use crate::common::{metres, Floor};
use lockstep_agents::{Alertness, MindEvent, Stimulus};
use lockstep_core::math::Fixed32;
use lockstep_core::Column;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_noise_makes_an_idle_agent_curious_with_half_confidence_that_fades() {
    let floor = Floor::new(40, 20, &[(5, 5)]);
    let agent = floor.bodies[0];
    let mut minds = floor.minds();
    let leashes = Column::new();
    let heard = [(
        agent,
        Stimulus::Heard {
            at: floor.cell(10, 5),
            source: None,
        },
    )];
    let events = floor.think(&mut minds, &leashes, &heard, Fixed32::ZERO, 8);
    assert_eq!(
        events,
        vec![MindEvent::Changed {
            agent,
            from: Alertness::Idle,
            to: Alertness::Curious
        }]
    );
    let memory = minds.get(agent).unwrap().memory.unwrap();
    assert_eq!(memory.position, floor.cell(10, 5));
    assert_eq!(memory.confidence, Fixed32::HALF);
    // Half way to forgetting: a quarter.
    floor.think(&mut minds, &leashes, &[], metres(5), 8);
    let memory = minds.get(agent).unwrap().memory.unwrap();
    assert_eq!(memory.confidence, Fixed32::from_ratio(1, 4));
    assert_eq!(memory.age_minutes, metres(5));
}

#[test]
fn a_searching_agent_that_sees_again_is_alert_and_an_alert_one_ignores_noise() {
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
    // A noise elsewhere does not pull an Alert agent off its target.
    let noise = [(
        agent,
        Stimulus::Heard {
            at: floor.cell(30, 5),
            source: None,
        },
    )];
    floor.think(&mut minds, &leashes, &noise, Fixed32::ZERO, 8);
    let mind = minds.get(agent).unwrap();
    assert_eq!(mind.alertness, Alertness::Alert);
    assert_eq!(mind.memory.unwrap().position, floor.cell(15, 5));
    floor.think(&mut minds, &leashes, &[], metres(3), 8);
    assert_eq!(minds.get(agent).unwrap().alertness, Alertness::Searching);
    let events = floor.think(&mut minds, &leashes, &saw, Fixed32::ZERO, 8);
    assert_eq!(
        events,
        vec![MindEvent::Changed {
            agent,
            from: Alertness::Searching,
            to: Alertness::Alert
        }]
    );
}

#[test]
fn a_louder_memory_is_not_replaced_by_a_quieter_noise() {
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
    floor.think(&mut minds, &leashes, &[], metres(3), 8);
    // Searching, the sighting at 0.7 confidence; a noise at 0.5 does not replace it.
    let noise = [(
        agent,
        Stimulus::Heard {
            at: floor.cell(30, 5),
            source: None,
        },
    )];
    floor.think(&mut minds, &leashes, &noise, Fixed32::ZERO, 8);
    assert_eq!(
        minds.get(agent).unwrap().memory.unwrap().position,
        floor.cell(15, 5)
    );
}
