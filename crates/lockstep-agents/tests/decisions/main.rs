// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod a_blocked_mover_slides_past_within_two_steps;
mod a_horde_crosses_a_doorway;
mod director_never_exceeds_its_budget;
mod memory_fades_to_searching_then_idle;
mod staggered_checks;
mod wind_ranges;
