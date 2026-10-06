// SPDX-License-Identifier: Apache-2.0
//! Decision: Silent Bells uses `Square8` on half-metre cells, with octile distance (10 per straight
//! step, 14 per diagonal), because its streets and walls are rectangular.
//! Alternative rejected: hexagons, which fit round shapes but not rectangular streets, and
//! `Square4`, whose Manhattan distance is far too long along diagonals.
//! Would change if: the octile distance is more than 8 percent away from the true distance
//! between two cell centres anywhere on a circle of radius 50 (the number to beat), or `Square4`
//! is within 30 percent.
//!
//! The measure is the worst relative error over every pair of points on the circle, comparing the
//! topology distance with the true distance between the two cell centres.
//!
//! About the hexagon figure in the reference ("at most 3 percent"): with true hexagonal geometry,
//! the cube distance over-counts by up to 15.5 percent along the directions halfway between two
//! lattice axes, so no hexagon distance can reach 3 percent. The test records the measured value
//! and docs/decisions/0005-spatial.md raises this for the owner.

use lockstep_core::math::unit_circle_table;
use lockstep_spatial::{Cell, Square4, Square8, Topology};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const WIDTH: u32 = 121;

/// Cells on a circle of radius 50 centred at (60, 60), one per degree.
fn circle() -> Vec<(i32, i32)> {
    (0..360)
        .map(|index| unit_circle_table(index, 360, 50))
        .map(|(x, y)| (x + 60, y + 60))
        .collect()
}

fn worst_error(
    distance: impl Fn(Cell, Cell) -> u32,
    position: impl Fn((i32, i32)) -> (f64, f64),
) -> f64 {
    let points = circle();
    let mut worst: f64 = 0.0;
    for (index, a) in points.iter().enumerate() {
        for b in &points[index + 1..] {
            let ((ax, ay), (bx, by)) = (position(*a), position(*b));
            let true_distance = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            if true_distance < 10.0 {
                continue; // rounding to whole cells dominates the error for tiny chords
            }
            let cell = |(x, y): (i32, i32)| Cell(y as u32 * WIDTH + x as u32);
            let measured = distance(cell(*a), cell(*b)) as f64 / 10.0;
            worst = worst.max((measured / true_distance - 1.0).abs());
        }
    }
    worst * 100.0
}

#[test]
fn octile_distance_stays_within_eight_percent_of_the_true_distance() {
    let worst = worst_error(
        |a, b| Square8::distance(a, b, WIDTH),
        |(x, y)| (x as f64, y as f64),
    );
    eprintln!("octile worst error: {worst:.2} percent");
    assert!(worst <= 8.0, "worst error {worst:.2} percent");
    assert!(
        worst >= 5.0,
        "the measure should see the octile error, got {worst:.2} percent"
    );
}

#[test]
fn manhattan_distance_is_at_least_thirty_percent_off() {
    let worst = worst_error(
        |a, b| Square4::distance(a, b, WIDTH),
        |(x, y)| (x as f64, y as f64),
    );
    eprintln!("manhattan worst error: {worst:.2} percent");
    assert!(worst >= 30.0, "worst error {worst:.2} percent");
}

#[cfg(feature = "hex")]
#[test]
fn hex_distance_is_at_best_fifteen_percent_off_in_true_geometry() {
    use lockstep_spatial::Hex;
    // Odd rows sit half a cell to the right and rows are 0.866 apart, so every neighbour is one unit away.
    let position = |(x, y): (i32, i32)| {
        (
            x as f64 + 0.5 * (y & 1) as f64,
            y as f64 * 3f64.sqrt() / 2.0,
        )
    };
    let worst = worst_error(|a, b| Hex::distance(a, b, WIDTH), position);
    eprintln!("hex worst error: {worst:.2} percent");
    assert!(
        (10.0..=16.0).contains(&worst),
        "worst error {worst:.2} percent"
    );
}
