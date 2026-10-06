use crate::common::{probe_runner, Add, Applied, STEP_SECONDS};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn intents_queued_between_steps_apply_to_the_next_step_that_runs() {
    let mut runner = probe_runner(1);
    runner.queue([Add(1), Add(2)]);
    assert_eq!(runner.advance(STEP_SECONDS / 4.0).steps_run, 0);
    runner.queue([Add(3)]);
    assert_eq!(runner.advance(STEP_SECONDS / 4.0).steps_run, 0);

    let advanced = runner.advance(STEP_SECONDS);
    assert_eq!(advanced.steps_run, 1);
    assert_eq!(advanced.events, vec![Applied(1), Applied(2), Applied(3)]);
    assert_eq!(runner.snapshot().total, 6);
}

#[test]
fn queued_intents_apply_exactly_once_and_only_to_the_first_step_run() {
    let mut runner = probe_runner(1);
    runner.queue([Add(10)]);
    let advanced = runner.advance(STEP_SECONDS * 3.0 + STEP_SECONDS / 2.0);
    assert_eq!(advanced.steps_run, 3);
    assert_eq!(advanced.events, vec![Applied(10)]);
    assert_eq!(runner.snapshot().total, 10);
}

#[test]
fn step_once_bypasses_the_queue_and_leaves_it_pending() {
    let mut runner = probe_runner(1);
    runner.queue([Add(4)]);
    let stepped = runner.step_once(&[Add(1)]);
    assert_eq!(stepped.events, vec![Applied(1)]);

    let advanced = runner.advance(STEP_SECONDS);
    assert_eq!(advanced.events, vec![Applied(4)]);
    assert_eq!(runner.snapshot().total, 5);
}
