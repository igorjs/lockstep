// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod a_binding_affix_holds_until_repaired;
mod a_put_never_splits_an_item;
mod a_stack_spoils_as_its_stalest_unit;
mod spoilage_ignores_the_step_rate;
