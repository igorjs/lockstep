// SPDX-License-Identifier: Apache-2.0
use crate::perception::{distance_metres, Senses};
use lockstep_combat::direction;
use lockstep_core::math::{cos, Fixed32, Turn};
use lockstep_core::{Column, Handle};
use lockstep_spatial::{line_of_sight, Cell, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// Raw 16.16 one, squared: the scale of a product of two raw values.
const ONE_SQUARED: i128 = 1 << 32;

/// The wind: where it blows toward, and how hard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wind {
    /// The direction the wind blows toward, counter-clockwise from east.
    pub direction: Turn,
    pub strength_metres_per_second: Fixed32,
}

/// Something heard: where, how loud (the distance it carries in still air), and who made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noise {
    pub at: Cell,
    pub loudness_metres: Fixed32,
    pub source: Option<Handle>,
    /// Heard in the mind, not the air: the wind neither carries nor masks it.
    pub psychic: bool,
}

/// One listener hearing one noise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heard {
    pub listener: Handle,
    pub at: Cell,
    pub source: Option<Handle>,
    pub distance_metres: Fixed32,
}

/// How far a noise carries toward a listener in a direction from the source:
/// `loudness × (1 + 0.04 × strength × cos θ)`, θ between the wind and the direction to the
/// listener, in exact integers rounded once. A 6-metre footstep in a 10 metre a second wind
/// carries 8.4 metres downwind, 3.6 upwind and 6 across. Never below zero.
pub fn effective_range(loudness_metres: Fixed32, wind: Wind, toward_listener: Turn) -> Fixed32 {
    let cosine = cos(toward_listener.wrapping_sub(wind.direction)).raw() as i128;
    // A negative strength is still air, as in `hearing_threshold`.
    let strength = wind.strength_metres_per_second.raw().max(0) as i128;
    // (100 + 4 × s × cos) / 100, with s and cos both raw.
    let numerator = 100 * ONE_SQUARED + 4 * strength * cosine;
    let range = divide_rounded(loudness_metres.raw() as i128 * numerator, 100 * ONE_SQUARED);
    Fixed32::from_raw(range.clamp(0, i32::MAX as i128) as i32)
}

/// How much the wind raises every listener's hearing threshold: 0.3 metres per metre a second.
pub fn hearing_threshold(wind: Wind) -> Fixed32 {
    let raw = divide_rounded(wind.strength_metres_per_second.raw().max(0) as i128 * 3, 10);
    Fixed32::from_raw(raw as i32)
}

/// How far from its source a listener in a direction hears a noise in the open: the effective
/// range less the hearing threshold, never below zero. A psychic noise ignores the wind and is
/// heard to its loudness.
pub fn audible_metres(noise: &Noise, wind: Wind, toward_listener: Turn) -> Fixed32 {
    if noise.psychic {
        return noise.loudness_metres.max(Fixed32::ZERO);
    }
    let range = effective_range(noise.loudness_metres, wind, toward_listener);
    (range - hearing_threshold(wind)).max(Fixed32::ZERO)
}

/// Every body with `Senses` that hears a noise, sorted by handle, never the source. A listener
/// hears it within `audible_metres` toward it, halved when it has no line of sight to the noise
/// from its own eye height (a wall or a rise between muffles it), and within its own hearing
/// range.
pub fn hear<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    senses: &Column<Senses>,
    noise: &Noise,
    wind: Wind,
    cell_metres: Fixed32,
    out: &mut Vec<Heard>,
) {
    out.clear();
    if cell_metres <= Fixed32::ZERO {
        return;
    }
    let loudest = if noise.psychic {
        noise.loudness_metres
    } else {
        effective_range(noise.loudness_metres, wind, wind.direction)
    };
    // Every cell a listener could hear from, in tenths of a cell as `within` counts. Off the
    // axes a topology's step distance runs longer than the straight one, up to the square root
    // of two on `Square4`, so the radius is half as long again, plus two cells.
    let cells = loudest.raw().max(0) as i64 / cell_metres.raw() as i64 + 1;
    let radius = (cells * 15 + 20).min(u32::MAX as i64) as u32;
    let mut nearby = Vec::new();
    occupancy.within(map, noise.at, radius, &mut nearby);
    let mut line = Vec::new();
    for (cell, listener) in nearby {
        if Some(listener) == noise.source {
            continue;
        }
        let Some(listener_senses) = senses.get(listener) else {
            continue;
        };
        let distance = distance_metres(map, noise.at, cell, cell_metres);
        let mut reach = audible_metres(noise, wind, direction(map, noise.at, cell));
        // Muffled when the listener, at its own ear height, has no line to the noise.
        if !line_of_sight(map, cell, noise.at, listener_senses.eye_height, &mut line) {
            reach = Fixed32::from_raw(reach.raw() / 2);
        }
        if distance <= reach && distance <= listener_senses.hearing_range_metres {
            out.push(Heard {
                listener,
                at: noise.at,
                source: noise.source,
                distance_metres: distance,
            });
        }
    }
    out.sort_by_key(|heard| heard.listener);
    out.dedup_by_key(|heard| heard.listener);
}

/// `numerator / denominator`, rounding halves away from zero. The denominator is positive.
fn divide_rounded(numerator: i128, denominator: i128) -> i128 {
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    }
}
