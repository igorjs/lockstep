// SPDX-License-Identifier: Apache-2.0
//! Decision: a choice scores the plain integer sum of its considerations, the highest total wins
//! and a tie goes to the lowest id. A delegated task is accepted, delayed or refused by its total
//! against two thresholds, and a refusal names the consideration that scored lowest.
//! Alternative rejected: weighted floating point scores, which differ across platforms; and a
//! refusal with no reason, which leaves the host nothing to show.
//! Would change if: two choices of equal total pick the higher id, or a refused task names
//! anything but its lowest-scoring consideration.

use lockstep_agents::{choose, evaluate_task, Choice, Reason, Task, TaskResponse};
use lockstep_core::{Handle, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// What the considerations read: trust in the one asking, and the agent's fear.
struct World {
    trust: i32,
    fear: i32,
}

fn agent() -> Handle {
    StableVector::new().insert(())
}

#[test]
fn equal_totals_go_to_the_lowest_id() {
    let choice = |id: u16, score: i32| Choice::<World> {
        id,
        considerations: vec![Box::new(move |_: Handle, _: &World| score)],
    };
    let world = World { trust: 0, fear: 0 };
    let choices = vec![choice(7, 10), choice(3, 10), choice(5, 9)];
    assert_eq!(choose(agent(), &choices, &world), Some(3));
}

#[test]
fn a_task_is_accepted_delayed_or_refused_for_its_weakest_reason() {
    const TRUST: Reason = Reason(1);
    const FEAR: Reason = Reason(2);
    let task = Task::<World> {
        id: 1,
        considerations: vec![
            (TRUST, Box::new(|_: Handle, world: &World| world.trust)),
            (FEAR, Box::new(|_: Handle, world: &World| -world.fear)),
        ],
        accept_at: 50,
        delay_at: 20,
        delay_minutes: 30,
    };
    let answer = |trust, fear| evaluate_task(agent(), &task, &World { trust, fear });
    assert_eq!(answer(80, 10), TaskResponse::Accept);
    assert_eq!(answer(40, 10), TaskResponse::Delay { minutes: 30 });
    // Too scared: fear scores lowest.
    assert_eq!(answer(30, 40), TaskResponse::Refuse { reason: FEAR });
    // Too little trust: trust scores lowest.
    assert_eq!(answer(-20, 0), TaskResponse::Refuse { reason: TRUST });
}
