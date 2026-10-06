// SPDX-License-Identifier: Apache-2.0
//! Decision: a noise carries `loudness × (1 + 0.04 × strength × cos θ)` metres, computed in exact
//! integers and rounded once, and a listener hears it within that range less the wind's hearing
//! threshold of 0.3 metres per metre a second. A psychic noise ignores the wind both ways.
//! Alternative rejected: subtracting the threshold before the wind scales the noise, which
//! breaks the reference's 8.4, 3.6 and 6.0 metre ranges; and a gale that silences every
//! footstep, which no single threshold does for footsteps up to 9 metres.
//! Would change if: a 6 metre footstep at 10 metres a second carries anything but exactly 8.4,
//! 3.6 and 6.0 metres, or a 14 metre a second gale lets a crosswind footstep be heard beyond 2
//! metres.

use crate::common::metres;
use lockstep_agents::{audible_metres, effective_range, hearing_threshold, Noise, Wind};
use lockstep_core::math::Fixed32;
use lockstep_spatial::Cell;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const EAST: u16 = 0;
const NORTH: u16 = 16_384;
const WEST: u16 = 32_768;

fn wind(strength: i32) -> Wind {
    Wind {
        direction: EAST,
        strength_metres_per_second: metres(strength),
    }
}

fn footstep(psychic: bool) -> Noise {
    Noise {
        at: Cell(0),
        loudness_metres: metres(6),
        source: None,
        psychic,
    }
}

#[test]
fn a_six_metre_footstep_in_a_ten_metre_wind_carries_8_4_down_3_6_up_and_6_across() {
    let footstep = metres(6);
    assert_eq!(
        effective_range(footstep, wind(10), EAST),
        Fixed32::from_ratio(84, 10)
    );
    assert_eq!(
        effective_range(footstep, wind(10), WEST),
        Fixed32::from_ratio(36, 10)
    );
    assert_eq!(effective_range(footstep, wind(10), NORTH), metres(6));
    assert_eq!(
        effective_range(footstep, wind(0), EAST),
        metres(6),
        "still air"
    );
}

#[test]
fn a_fourteen_metre_gale_masks_walking() {
    let gale = wind(14);
    assert_eq!(hearing_threshold(gale), Fixed32::from_ratio(42, 10));
    // Across the wind a 6 metre footstep is heard to 1.8 metres, against the wind not at all.
    assert_eq!(
        audible_metres(&footstep(false), gale, NORTH),
        Fixed32::from_ratio(18, 10)
    );
    assert_eq!(audible_metres(&footstep(false), gale, WEST), Fixed32::ZERO);
    // A prayer ping ignores the wind: 6 metres every way.
    for toward in [EAST, NORTH, WEST] {
        assert_eq!(audible_metres(&footstep(true), gale, toward), metres(6));
    }
}
