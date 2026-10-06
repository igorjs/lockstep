// SPDX-License-Identifier: Apache-2.0
use lockstep_agents::{choose, evaluate_task, Choice, Consideration, Reason, Task, TaskResponse};
use lockstep_core::{Handle, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// A consideration as a type, beside the closures.
struct Hunger;

impl Consideration<i32> for Hunger {
    fn score(&self, _: Handle, hunger: &i32) -> i32 {
        *hunger * 2
    }
}

#[test]
fn scores_show_each_consideration_and_totals_never_wrap() {
    let agent = StableVector::new().insert(());
    let eat = Choice::<i32> {
        id: 1,
        considerations: vec![Box::new(Hunger), Box::new(|_: Handle, _: &i32| -5)],
    };
    assert_eq!(eat.scores(agent, &30), vec![60, -5]);
    assert_eq!(eat.total(agent, &30), 55);
    let huge = Choice::<i32> {
        id: 2,
        considerations: vec![
            Box::new(|_: Handle, _: &i32| i32::MAX),
            Box::new(|_: Handle, _: &i32| i32::MAX),
        ],
    };
    assert_eq!(huge.total(agent, &0), 2 * i32::MAX as i64);
    assert_eq!(choose(agent, &[eat, huge], &30), Some(2));
    assert_eq!(choose::<i32>(agent, &[], &0), None);
}

#[test]
fn a_task_with_no_considerations_scores_zero() {
    let agent = StableVector::new().insert(());
    let task = Task::<i32> {
        id: 1,
        considerations: Vec::new(),
        accept_at: 1,
        delay_at: 1,
        delay_minutes: 0,
    };
    assert_eq!(
        evaluate_task(agent, &task, &0),
        TaskResponse::Refuse { reason: Reason(0) }
    );
}
