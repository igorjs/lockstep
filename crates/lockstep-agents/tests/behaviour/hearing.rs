// SPDX-License-Identifier: Apache-2.0
use crate::common::{half_metre, metres, Floor, EAST};
use lockstep_agents::{hear, Heard, Noise, Wind};

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
}

/// A 6 metre noise and one listener 8 cells east and 8 north: 5.66 metres away in a straight
/// line, though `Square4` counts 16 steps.
fn diagonal_listener_hears<T: lockstep_spatial::Topology>() -> bool {
    let map: lockstep_spatial::GridMap<T> = lockstep_spatial::GridMap::new(40, 40);
    let mut occupancy = lockstep_spatial::Occupancy::new(&map);
    let mut store = lockstep_core::StableVector::new();
    let listener = store.insert(());
    occupancy.place(map.index(28, 12), listener).unwrap();
    let mut senses = lockstep_core::Column::new();
    senses.set(listener, crate::common::senses());
    let noise = Noise {
        at: map.index(20, 20),
        loudness_metres: metres(6),
        source: None,
        psychic: false,
    };
    let mut heard = Vec::new();
    hear(
        &map,
        &occupancy,
        &senses,
        &noise,
        still(),
        half_metre(),
        &mut heard,
    );
    heard.iter().any(|heard| heard.listener == listener)
}

#[test]
fn a_listener_off_the_axes_hears_on_every_square_topology() {
    assert!(diagonal_listener_hears::<lockstep_spatial::Square4>());
    assert!(diagonal_listener_hears::<lockstep_spatial::Square8>());
}

#[test]
fn a_tall_listener_hears_over_a_low_wall_unmuffled() {
    // A low wall right in front of a listener whose ears are above it: the noise 5 metres off is
    // heard in full; a listener with ears below the wall hears it halved, so not at all.
    let mut floor = Floor::new(60, 20, &[(30, 10)]);
    floor.map.set_low_wall(floor.cell(29, 10), 2);
    let listener = floor.bodies[0];
    floor.senses.get_mut(listener).unwrap().eye_height = 3;
    let noise = Noise {
        at: floor.cell(20, 10),
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
    assert_eq!(heard.len(), 1);
    floor.senses.get_mut(listener).unwrap().eye_height = 1;
    hear(
        &floor.map,
        &floor.occupancy,
        &floor.senses,
        &noise,
        still(),
        half_metre(),
        &mut heard,
    );
    assert!(heard.is_empty());
}

#[test]
fn a_negative_wind_strength_is_still_air() {
    let reversed = Wind {
        direction: EAST,
        strength_metres_per_second: metres(-10),
    };
    for toward in [0u16, 16_384, 32_768] {
        assert_eq!(
            lockstep_agents::effective_range(metres(6), reversed, toward),
            metres(6)
        );
    }
}
