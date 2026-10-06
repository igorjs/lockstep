// SPDX-License-Identifier: Apache-2.0
use crate::common::{probe_runner, Add, STEP_SECONDS};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Frame lengths are multiples of one 128th of a second, so every partial sum is exact in a
/// 32-bit float and the only rounding comes from the step length itself. The total is chosen
/// away from a step boundary.
#[test]
fn irregular_frame_times_produce_the_same_step_count_and_hash_as_regular_ones() {
    let total_frames_of_one_sixty_fourth = 641; // 10.015625 seconds, 300.47 steps

    let mut regular = probe_runner(7);
    regular.queue([Add(5)]);
    let mut regular_steps = 0;
    for _ in 0..total_frames_of_one_sixty_fourth {
        regular_steps += regular.advance(1.0 / 64.0).steps_run;
    }

    let mut irregular = probe_runner(7);
    irregular.queue([Add(5)]);
    let pattern: [f32; 6] = [
        1.0 / 128.0,
        5.0 / 128.0,
        1.0 / 32.0,
        3.0 / 128.0,
        1.0 / 16.0,
        9.0 / 128.0,
    ];
    let mut remaining_in_128ths = 641 * 2;
    let mut irregular_steps = 0;
    let mut index = 0;
    while remaining_in_128ths > 0 {
        let frame = pattern[index % pattern.len()];
        let frame_in_128ths = (frame * 128.0) as i32;
        let used = frame_in_128ths.min(remaining_in_128ths);
        irregular_steps += irregular.advance(used as f32 / 128.0).steps_run;
        remaining_in_128ths -= used;
        index += 1;
    }

    assert_eq!(regular_steps, 300);
    assert_eq!(irregular_steps, regular_steps);
    assert_eq!(irregular.hash(), regular.hash());
}

#[test]
fn a_hitch_longer_than_eight_steps_drops_the_excess_and_never_spirals() {
    let mut runner = probe_runner(1);
    let hitch = runner.advance(10.0);
    assert_eq!(hitch.steps_run, 8);
    assert_eq!(runner.accumulator_fraction(), 0.0);

    let next = runner.advance(0.0);
    assert_eq!(next.steps_run, 0, "the excess time was dropped, not owed");
    assert_eq!(runner.step_number(), 8);
}

#[test]
fn step_once_and_advance_with_the_same_intents_produce_the_same_hash() {
    let mut stepped = probe_runner(3);
    let mut advanced = probe_runner(3);
    for step in 0..500 {
        let intents = if step % 7 == 0 {
            vec![Add(step)]
        } else {
            vec![]
        };
        stepped.step_once(&intents);
        advanced.queue(intents);
        let result = advanced.advance(STEP_SECONDS);
        assert_eq!(result.steps_run, 1);
    }
    assert_eq!(stepped.step_number(), advanced.step_number());
    assert_eq!(stepped.hash(), advanced.hash());
}

#[test]
fn negative_real_time_is_ignored() {
    let mut runner = probe_runner(1);
    assert_eq!(runner.advance(-5.0).steps_run, 0);
    assert_eq!(runner.step_number(), 0);
}
