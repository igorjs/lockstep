// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod control_never_falls_to_the_lost;
mod retired_names_stay_retired;
mod the_budget_is_spent_at_stated_costs;
mod the_chronicle_is_sealed_with_its_version;
