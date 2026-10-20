// SPDX-License-Identifier: Apache-2.0
//! Decision: the hexagon topology is kept with its measured error, 15.47 percent over the true
//! distance along the directions halfway between two lattice axes. The reference's figure of 3
//! percent is a recorded departure, not a target.
//! Alternative rejected: scaling or otherwise tuning the step count to reach 3 percent. The error
//! is not a scale: the ratio of steps to true distance runs from 1 along an axis to 2 / sqrt(3)
//! halfway between two axes, so even the best constant factor leaves 7.2 percent either way.
//! Would change if: the worst error over every pair of points on the circle of radius 50 leaves
//! 15.40 to 15.55 percent, or the best constant factor reaches under 7
//! percent. Either means the distance or the geometry changed.

#![cfg(feature = "hex")]

use lockstep_core::math::unit_circle_table;
use lockstep_spatial::{Cell, Hex, Topology};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const WIDTH: u32 = 121;

/// The ratio of step distance to true distance for every pair of points on a circle of radius 50
/// of hexagons, in true geometry: odd rows sit half a cell right and rows are 0.866 apart, so
/// every neighbour is one unit away.
fn ratios() -> Vec<f64> {
    let points: Vec<(i32, i32)> = (0..360)
        .map(|index| unit_circle_table(index, 360, 50))
        .map(|(x, y)| (x + 60, y + 60))
        .collect();
    let position = |(x, y): (i32, i32)| {
        (
            x as f64 + 0.5 * (y & 1) as f64,
            y as f64 * 3f64.sqrt() / 2.0,
        )
    };
    let mut ratios = Vec::new();
    for (index, a) in points.iter().enumerate() {
        for b in &points[index + 1..] {
            let ((ax, ay), (bx, by)) = (position(*a), position(*b));
            let true_distance = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            if true_distance < 10.0 {
                continue; // rounding to whole cells dominates the error for tiny chords
            }
            let cell = |(x, y): (i32, i32)| Cell(y as u32 * WIDTH + x as u32);
            let measured = Hex::distance(cell(*a), cell(*b), WIDTH) as f64 / 10.0;
            ratios.push(measured / true_distance);
        }
    }
    ratios
}

#[test]
fn the_worst_hexagon_error_is_fifteen_and_a_half_percent() {
    let worst = ratios()
        .iter()
        .map(|ratio| (ratio - 1.0).abs())
        .fold(0.0, f64::max)
        * 100.0;
    eprintln!("hexagon worst error: {worst:.2} percent");
    assert!(
        (15.40..=15.55).contains(&worst),
        "worst error {worst:.2} percent"
    );
}

#[test]
fn no_constant_factor_brings_the_hexagon_error_under_seven_percent() {
    let ratios = ratios();
    let low = ratios.iter().copied().fold(f64::MAX, f64::min);
    let high = ratios.iter().copied().fold(0.0, f64::max);
    // The best factor k balances k * low = 1 - e against k * high = 1 + e.
    let best = (high - low) / (high + low) * 100.0;
    eprintln!("ratios {low:.4} to {high:.4}, best constant factor leaves {best:.2} percent");
    assert!(best >= 7.0, "best constant factor leaves {best:.2} percent");
}
