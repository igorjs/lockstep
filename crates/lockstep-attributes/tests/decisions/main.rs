// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod add_then_multiply_then_override;
mod daily_budget;
mod derived_follows_inputs;
mod effects_determinism;
mod effects_tick_by_game_minutes;
mod ratchet;
mod refresh_does_not_stack;
mod scale_versus_clamp;
mod thresholds_fire_once_per_crossing;
