// SPDX-License-Identifier: Apache-2.0
//! One test per design decision. Each file states the decision, the alternative that was
//! rejected, and the number that would change the decision.
//!
//! A failing decision test is a design question: stop and report, never edit the test to pass.

#[path = "../common/mod.rs"]
mod common;

mod a_save_continues_a_recording;
mod bisect_finds_a_planted_divergence;
mod chance_in_basis_points;
mod clock_as_a_rate;
mod clock_in_integer_ticks;
mod derived_messages;
mod fixed_endian_hash;
mod fixed_point_math;
mod fixed_step;
mod game_minutes_add_up_exactly;
mod integer_cordic_tables;
mod money_shares_never_lose_a_unit;
mod movement_never_overshoots;
mod named_streams;
mod saves_migrate_one_version_at_a_time;
mod smoothing_table_committed;
mod stable_vector_over_slotmap_rows;
mod store_reuses_the_lowest_free_slot;
mod timeline_is_a_projection;
