// SPDX-License-Identifier: Apache-2.0
//! Decision: movement answers on the step it is ordered, running is exactly twice as loud as
//! walking, and running empties stamina in about 17 seconds (100 stamina at 6 a second).
//! Alternative rejected: starting to move only once a cell's worth of progress builds up, which
//! adds several steps of delay to every click; and noise as a free-form number per gait.
//! Would change if: a move order does not move the body on its own step, running is heard at
//! anything but twice the walking distance, or running empties stamina outside 16.5 to 17 seconds.

use crate::walkers::Walkers;
use lockstep_combat::{Gait, MoveOrder, MovementEvent};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_move_order_moves_the_body_on_the_step_it_arrives() {
    let mut walkers = Walkers::new(20, 3);
    let who = walkers.add(2, 1);
    let target = walkers.map.index(10, 1);
    let events = walkers.step(&[(
        who,
        MoveOrder::MoveTo {
            target,
            gait: Gait::Walk,
        },
    )]);
    assert!(
        matches!(events.as_slice(), [MovementEvent::Moved { .. }]),
        "{events:?}"
    );
    assert_eq!(walkers.at(who), (3, 1));
}

#[test]
fn running_is_heard_at_exactly_twice_the_walking_distance() {
    let noise = |gait| {
        let mut walkers = Walkers::new(20, 3);
        let who = walkers.add(2, 1);
        let target = walkers.map.index(10, 1);
        let events = walkers.step(&[(who, MoveOrder::MoveTo { target, gait })]);
        match events[0] {
            MovementEvent::Moved { noise_metres, .. } => noise_metres,
            _ => panic!("moved"),
        }
    };
    assert_eq!(noise(Gait::Walk), Fixed32::from_int(6));
    assert_eq!(noise(Gait::Run), Fixed32::from_int(12));
    assert_eq!(noise(Gait::Sneak), Fixed32::from_int(3));
}

#[test]
fn running_empties_stamina_in_about_seventeen_seconds() {
    let mut walkers = Walkers::new(250, 3);
    let who = walkers.add(1, 1);
    let target = walkers.map.index(248, 1);
    walkers.step(&[(
        who,
        MoveOrder::MoveTo {
            target,
            gait: Gait::Run,
        },
    )]);
    let mut steps = 1;
    loop {
        let events = walkers.step(&[]);
        steps += 1;
        if events.contains(&MovementEvent::Exhausted { who }) {
            break;
        }
        assert!(steps < 1_000, "never ran out");
    }
    let seconds = Fixed32::from_ratio(steps, 30);
    assert!(
        seconds >= Fixed32::from_ratio(33, 2) && seconds <= Fixed32::from_int(17),
        "{steps} steps"
    );
    assert_eq!(walkers.stamina(who), Fixed32::ZERO);
}
