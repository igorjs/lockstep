// SPDX-License-Identifier: Apache-2.0
use crate::common::{minutes, rules, Floor};
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
    floor.think(&mut minds, &leashes, &[], minutes(5), 8);
    let memory = minds.get(agent).unwrap().memory.unwrap();
    assert_eq!(memory.confidence, Fixed32::from_ratio(1, 4));
    assert_eq!(memory.age_minutes, minutes(5));
}

#[test]
fn an_alert_agent_ignores_a_noise_even_once_its_memory_has_faded() {
    // Out of sight for 6 of the 8 minutes it takes to search: the sighting is down to 0.4, below
    // a noise's 0.5, so only Alert keeps the noise from replacing it.
    let floor = Floor::new(40, 20, &[(5, 5), (15, 5)]);
    let (agent, target) = (floor.bodies[0], floor.bodies[1]);
    let mut slow = rules();
    slow.lose_sight_after_minutes = minutes(8);
    let mut minds = floor.minds();
    let leashes = Column::new();
    let saw = [(
        agent,
        Stimulus::Saw {
            target,
            at: floor.cell(15, 5),
        },
    )];
    floor.think_with(&slow, &mut minds, &leashes, &saw, Fixed32::ZERO, 8);
    floor.think_with(&slow, &mut minds, &leashes, &[], minutes(6), 8);
    let noise = [(
        agent,
        Stimulus::Heard {
            at: floor.cell(30, 5),
            source: None,
        },
    )];
    floor.think_with(&slow, &mut minds, &leashes, &noise, Fixed32::ZERO, 8);
    let mind = minds.get(agent).unwrap();
    assert_eq!(mind.alertness, Alertness::Alert);
    assert_eq!(mind.memory.unwrap().position, floor.cell(15, 5));
}

#[test]
fn a_searching_agent_that_sees_again_is_alert() {
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
    floor.think(&mut minds, &leashes, &[], minutes(3), 8);
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
fn the_director_ranks_by_the_freshest_sighting_not_a_stale_one() {
    // A budget of one. The first agent stands on the spot it saw the target a minute ago; the
    // second sees the target now, 3 cells away, where the first is 10 cells off.
    let floor = Floor::new(40, 20, &[(5, 5), (12, 5), (15, 5)]);
    let (stale, fresh, target) = (floor.bodies[0], floor.bodies[1], floor.bodies[2]);
    let mut minds = floor.minds();
    minds.unset(target);
    let leashes = Column::new();
    let old = [(
        stale,
        Stimulus::Saw {
            target,
            at: floor.cell(5, 5),
        },
    )];
    floor.think(&mut minds, &leashes, &old, Fixed32::ZERO, 1);
    floor.think(&mut minds, &leashes, &[], minutes(1), 1);
    let now = [(
        fresh,
        Stimulus::Saw {
            target,
            at: floor.cell(15, 5),
        },
    )];
    floor.think(&mut minds, &leashes, &now, Fixed32::ZERO, 1);
    assert_eq!(minds.get(fresh).unwrap().alertness, Alertness::Alert);
    assert_eq!(minds.get(stale).unwrap().alertness, Alertness::Curious);
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
    floor.think(&mut minds, &leashes, &[], minutes(3), 8);
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
