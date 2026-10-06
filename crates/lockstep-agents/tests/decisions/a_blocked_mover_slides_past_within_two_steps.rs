// SPDX-License-Identifier: Apache-2.0
//! Decision: a mover whose next cell is held tries the two neighbours either side of its way,
//! nearest the intended direction first, before waiting. Agents and fighters share the one
//! function, `lockstep_combat::sidestep`.
//! Alternative rejected: waiting until the way clears, which locks a crowd behind one body.
//! Would change if: a mover with a body in its way and a free diagonal is not past the body
//! within two steps.

use lockstep_agents::{steer, SteerEvent};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, StableVector};
use lockstep_spatial::{FlowField, GridMap, Occupancy, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_mover_behind_a_standing_body_is_past_it_in_two_steps() {
    let map: GridMap<Square8> = GridMap::new(20, 5);
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let (mover, standing) = (store.insert(()), store.insert(()));
    occupancy.place(map.index(2, 2), mover).unwrap();
    occupancy.place(map.index(3, 2), standing).unwrap();
    let field = FlowField::build(&map, &[map.index(19, 2)], u32::MAX);
    let leashes = Column::new();
    let mut events = Vec::new();
    for _ in 0..2 {
        steer(
            &map,
            &mut occupancy,
            &field,
            &[mover],
            &leashes,
            Fixed32::HALF,
            &mut events,
        );
    }
    assert!(events
        .iter()
        .all(|event| matches!(event, SteerEvent::Stepped { .. })));
    let (x, _) = map.coordinates(occupancy.cell_of(mover).unwrap());
    assert!(x > 3, "past the body at x = 3, now at x = {x}");
}
