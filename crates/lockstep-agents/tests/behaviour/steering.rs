// SPDX-License-Identifier: Apache-2.0
use crate::common::metres;
use lockstep_agents::{steer, Leash, SteerEvent};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, StableVector};
use lockstep_spatial::{FlowField, GridMap, Occupancy, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_leashed_agent_follows_the_field_to_its_leash_and_waits_there() {
    let map: GridMap<Square8> = GridMap::new(40, 5);
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let agent = store.insert(());
    occupancy.place(map.index(5, 2), agent).unwrap();
    let mut leashes = Column::new();
    leashes.set(
        agent,
        Leash {
            home: map.index(5, 2),
            radius_metres: metres(5),
        },
    );
    let field = FlowField::build(&map, &[map.index(39, 2)], u32::MAX);
    let mut events = Vec::new();
    for _ in 0..40 {
        steer(
            &map,
            &mut occupancy,
            &field,
            &[agent],
            &leashes,
            Fixed32::HALF,
            &mut events,
        );
        let (x, _) = map.coordinates(occupancy.cell_of(agent).unwrap());
        assert!(x <= 15, "never beyond 5 metres from home");
    }
    assert_eq!(map.coordinates(occupancy.cell_of(agent).unwrap()), (15, 2));
    assert_eq!(events.last(), Some(&SteerEvent::Waited { who: agent }));
}

#[test]
fn a_boxed_in_agent_waits_and_one_at_the_goal_stays_quiet() {
    // A corridor one cell high: the body ahead blocks, and no sidestep exists.
    let mut map: GridMap<Square8> = GridMap::new(10, 3);
    for x in 0..10 {
        map.set_passable(map.index(x, 0), false);
        map.set_passable(map.index(x, 2), false);
    }
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let (behind, ahead) = (store.insert(()), store.insert(()));
    occupancy.place(map.index(2, 1), behind).unwrap();
    occupancy.place(map.index(9, 1), ahead).unwrap();
    let field = FlowField::build(&map, &[map.index(9, 1)], u32::MAX);
    let leashes = Column::new();
    let mut events = Vec::new();
    for _ in 0..8 {
        steer(
            &map,
            &mut occupancy,
            &field,
            &[ahead, behind],
            &leashes,
            Fixed32::HALF,
            &mut events,
        );
    }
    assert_eq!(map.coordinates(occupancy.cell_of(behind).unwrap()), (8, 1));
    assert_eq!(events.last(), Some(&SteerEvent::Waited { who: behind }));
    assert!(
        events.iter().all(|event| match event {
            SteerEvent::Stepped { who, .. } | SteerEvent::Waited { who } => *who == behind,
        }),
        "the agent at the goal makes no events"
    );
}

#[test]
fn a_door_closed_after_the_field_was_built_is_stepped_around() {
    let mut map: GridMap<Square8> = GridMap::new(20, 5);
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let agent = store.insert(());
    occupancy.place(map.index(2, 2), agent).unwrap();
    let field = FlowField::build(&map, &[map.index(19, 2)], u32::MAX);
    let door = map.index(3, 2);
    map.set_passable(door, false);
    let leashes = Column::new();
    let mut events = Vec::new();
    steer(
        &map,
        &mut occupancy,
        &field,
        &[agent],
        &leashes,
        Fixed32::HALF,
        &mut events,
    );
    let at = occupancy.cell_of(agent).unwrap();
    assert_ne!(at, door, "never into the wall");
    assert!(map.is_passable(at));
}

#[test]
fn an_agent_outside_its_leash_walks_back_toward_home() {
    // Home at (5, 2) with a 2 metre leash; the agent starts 5 metres out, and the field leads home.
    let map: GridMap<Square8> = GridMap::new(30, 5);
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let agent = store.insert(());
    occupancy.place(map.index(15, 2), agent).unwrap();
    let mut leashes = Column::new();
    leashes.set(
        agent,
        Leash {
            home: map.index(5, 2),
            radius_metres: metres(2),
        },
    );
    let field = FlowField::build(&map, &[map.index(5, 2)], u32::MAX);
    let mut events = Vec::new();
    for _ in 0..10 {
        steer(
            &map,
            &mut occupancy,
            &field,
            &[agent],
            &leashes,
            Fixed32::HALF,
            &mut events,
        );
    }
    assert_eq!(map.coordinates(occupancy.cell_of(agent).unwrap()), (5, 2));
}

#[test]
fn a_sidestep_outside_the_leash_gives_way_to_the_other_side() {
    // Heading east with a body ahead at (6, 2). The leash, centred below, allows the south-east
    // diagonal (6, 3) but not the north-east one (6, 1), which the sidestep tries first.
    let map: GridMap<Square8> = GridMap::new(20, 7);
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let (agent, standing) = (store.insert(()), store.insert(()));
    occupancy.place(map.index(5, 2), agent).unwrap();
    occupancy.place(map.index(6, 2), standing).unwrap();
    let mut leashes = Column::new();
    leashes.set(
        agent,
        Leash {
            home: map.index(5, 4),
            radius_metres: Fixed32::from_ratio(12, 10),
        },
    );
    let field = FlowField::build(&map, &[map.index(19, 2)], u32::MAX);
    let mut events = Vec::new();
    steer(
        &map,
        &mut occupancy,
        &field,
        &[agent],
        &leashes,
        Fixed32::HALF,
        &mut events,
    );
    assert_eq!(map.coordinates(occupancy.cell_of(agent).unwrap()), (6, 3));
}
