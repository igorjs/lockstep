// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod a_source_counts_once;
mod predicates_are_data_with_a_stable_hash;
mod rules_fire_once;
