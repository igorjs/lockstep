// SPDX-License-Identifier: Apache-2.0
use crate::common::{half_metre, metres, Floor, EAST};
use lockstep_agents::{hear, Heard, Noise, Wind};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn still() -> Wind {
    Wind::default()
}

#[test]
fn listeners_within_the_noise_hear_it_and_the_source_does_not() {
    // A 6 metre noise at (20, 10) made by the first body. Listeners at 5.5, 6 and 6.5 metres.
    let floor = Floor::new(50, 20, &[(20, 10), (31, 10), (32, 10), (33, 10)]);
    let source = floor.bodies[0];
    let noise = Noise {
        at: floor.cell(20, 10),
        loudness_metres: metres(6),
        source: Some(source),
        psychic: false,
    };
    let mut heard = Vec::new();
    hear(
        &floor.map,
        &floor.occupancy,
        &floor.senses,
        &noise,
        still(),
        half_metre(),
        &mut heard,
    );
    let listeners: Vec<_> = heard.iter().map(|heard| heard.listener).collect();
    assert_eq!(listeners, vec![floor.bodies[1], floor.bodies[2]]);
    assert_eq!(
        heard[1],
        Heard {
            listener: floor.bodies[2],
            at: noise.at,
            source: Some(source),
            distance_metres: metres(6),
        }
    );
}

#[test]
fn the_wind_carries_a_noise_downwind_less_its_threshold() {
    // 10 metres a second east: a 6 metre noise carries 8.4 east, heard to 8.4 less 3 = 5.4.
    let floor = Floor::new(60, 20, &[(30, 10), (40, 10), (41, 10), (20, 10)]);
    let noise = Noise {
        at: floor.cell(30, 10),
        loudness_metres: metres(6),
        source: Some(floor.bodies[0]),
        psychic: false,
    };
    let wind = Wind {
        direction: EAST,
        strength_metres_per_second: metres(10),
    };
    let mut heard = Vec::new();
    hear(
        &floor.map,
        &floor.occupancy,
        &floor.senses,
        &noise,
        wind,
        half_metre(),
        &mut heard,
    );
    let listeners: Vec<_> = heard.iter().map(|heard| heard.listener).collect();
    // 5 metres downwind heard, 5.5 not; 5 metres upwind not (3.6 less 3 is 0.6).
    assert_eq!(listeners, vec![floor.bodies[1]]);
}

#[test]
fn a_wall_between_halves_the_reach_and_the_hearing_range_caps_it() {
    let mut floor = Floor::new(60, 20, &[(10, 10), (16, 10), (22, 10)]);
    floor.map.set_passable(floor.cell(13, 10), false);
    let noise = Noise {
        at: floor.cell(10, 10),
        loudness_metres: metres(6),
        source: None,
        psychic: false,
    };
    let mut heard = Vec::new();
    hear(
        &floor.map,
        &floor.occupancy,
        &floor.senses,
        &noise,
        still(),
        half_metre(),
        &mut heard,
    );
    // Behind the wall: 3 metres away, heard within 6 / 2 = 3.
    let listeners: Vec<_> = heard.iter().map(|heard| heard.listener).collect();
    assert_eq!(listeners, vec![floor.bodies[0], floor.bodies[1]]);
    // A listener whose hearing reaches only 2 metres hears nothing 3 metres off.
    let deaf = floor.bodies[1];
    floor.senses.get_mut(deaf).unwrap().hearing_range_metres = metres(2);
    hear(
        &floor.map,
        &floor.occupancy,
        &floor.senses,
        &noise,
        still(),
        half_metre(),
        &mut heard,
    );
    assert!(heard.iter().all(|heard| heard.listener != deaf));
    let _ = Fixed32::ZERO;
}
