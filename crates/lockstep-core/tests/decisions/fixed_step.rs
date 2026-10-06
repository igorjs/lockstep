// SPDX-License-Identifier: Apache-2.0
//! Decision: the simulation advances in fixed steps, 30 per real second.
//! Alternative rejected: a variable step that follows the frame time.
//! Would change if: the fixed step cannot keep the hash identical across frame rates, or a
//! genre needs more than 30 steps per second for input feel (the number to beat is 30).

use crate::common::probe_runner;
use lockstep_core::StepConfiguration;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_default_is_thirty_steps_per_second_with_a_cap_of_eight_per_advance() {
    let configuration = StepConfiguration::default();
    assert_eq!(configuration.step_seconds, 1.0 / 30.0);
    assert_eq!(configuration.maximum_steps_per_advance, 8);
}

#[test]
fn the_hash_does_not_depend_on_the_frame_rate() {
    // 1.0166 seconds is 30.5 steps: away from a step boundary, so float rounding cannot matter.
    let mut at_sixty = probe_runner(5);
    let mut at_twenty = probe_runner(5);
    let mut at_sixty_steps = 0;
    let mut at_twenty_steps = 0;
    for _ in 0..61 {
        at_sixty_steps += at_sixty.advance(1.0 / 60.0).steps_run;
    }
    for _ in 0..20 {
        at_twenty_steps += at_twenty.advance(1.0 / 20.0).steps_run;
    }
    at_twenty_steps += at_twenty.advance(1.0 / 60.0).steps_run;
    assert_eq!(at_sixty_steps, 30);
    assert_eq!(at_twenty_steps, 30);
    assert_eq!(at_sixty.hash(), at_twenty.hash());
}
