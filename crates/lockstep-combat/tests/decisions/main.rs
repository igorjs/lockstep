// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../support/common.rs"]
mod common;
#[path = "../support/duel.rs"]
mod duel;

mod actions_and_dodges;
mod arc_cells_per_topology;
mod armour_before_resistance;
mod knockback_stops_at_walls;
