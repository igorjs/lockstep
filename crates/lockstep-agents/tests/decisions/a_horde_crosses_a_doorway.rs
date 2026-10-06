// SPDX-License-Identifier: Apache-2.0
//! Decision: steering is reservation over occupancy. Each agent, in handle order, moves into the
//! flow field's next cell or the shared sidestep, through occupancy, so the cell is taken at once
//! and no later agent in the step can take it.
//! Alternative rejected: computing every agent's move first and applying them together, which
//! needs a separate conflict pass and lets two agents pick the same cell.
//! Would change if: fewer than all 200 agents crossing a one-cell doorway are through in 2,000
//! steps (without the sidestep they jam: this is the check that pins it). Occupancy refuses a
//! held cell outright, so the per-step check that no two agents share a cell guards the
//! invariant rather than the decision.

use lockstep_agents::steer;
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, StableVector};
use lockstep_spatial::{FlowField, GridMap, Occupancy, Square8};
use std::collections::BTreeSet;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn two_hundred_agents_cross_a_doorway_without_sharing_a_cell() {
    // Two 20 by 20 rooms either side of a wall at x = 20 with a one-cell door at (20, 10).
    let mut map: GridMap<Square8> = GridMap::new(41, 20);
    for y in 0..20 {
        if y != 10 {
            map.set_passable(map.index(20, y), false);
        }
    }
    let mut occupancy = Occupancy::new(&map);
    let mut store = StableVector::new();
    let mut agents = Vec::new();
    for y in 0..20 {
        for x in 0..10 {
            let agent = store.insert(());
            occupancy.place(map.index(x, y), agent).unwrap();
            agents.push(agent);
        }
    }
    // The far wall of the second room: arrivals pack in from the back, so nobody stops in the
    // doorway's way.
    let goal: Vec<_> = (0..20).map(|y| map.index(40, y)).collect();
    let field = FlowField::build(&map, &goal, u32::MAX);
    let leashes = Column::new();
    let mut events = Vec::new();
    let mut through = 0;
    for step in 0..2_000 {
        events.clear();
        steer(
            &map,
            &mut occupancy,
            &field,
            &agents,
            &leashes,
            Fixed32::HALF,
            &mut events,
        );
        let mut cells = BTreeSet::new();
        for agent in &agents {
            let cell = occupancy.cell_of(*agent).unwrap();
            assert!(cells.insert(cell), "step {step}: two agents on {cell:?}");
        }
        through = agents
            .iter()
            .filter(|agent| map.coordinates(occupancy.cell_of(**agent).unwrap()).0 > 20)
            .count();
        if through == agents.len() {
            break;
        }
    }
    assert_eq!(through, 200);
}
