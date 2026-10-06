// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]
//! Bodies on a long map, for the movement tests.

use lockstep_combat::{
    step_movement, Defence, Fighter, MoveOrder, MovementEvent, MovementRules, MovementWorld, Mover,
    MovesetId,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, StableVector};
use lockstep_spatial::{Cell, GridMap, Occupancy, Pathfinder, Square8};

pub struct Walkers {
    pub map: GridMap<Square8>,
    pub occupancy: Occupancy,
    pub pathfinder: Pathfinder,
    pub movers: Column<Mover>,
    pub fighters: Column<Fighter>,
    pub rules: MovementRules,
    pub store: StableVector<()>,
    /// Steps a second.
    pub rate: u32,
}

impl Walkers {
    pub fn new(width: u32, height: u32) -> Self {
        let map: GridMap<Square8> = GridMap::new(width, height);
        Walkers {
            occupancy: Occupancy::new(&map),
            pathfinder: Pathfinder::new(&map),
            map,
            movers: Column::new(),
            fighters: Column::new(),
            rules: MovementRules::default(),
            store: StableVector::new(),
            rate: 30,
        }
    }

    /// A body with a mover and 100 stamina.
    pub fn add(&mut self, x: u32, y: u32) -> Handle {
        let who = self.store.insert(());
        self.occupancy.place(self.map.index(x, y), who).unwrap();
        self.movers.set(who, Mover::default());
        self.fighters.set(
            who,
            Fighter::new(MovesetId(0), Fixed32::from_int(100), Defence::default()),
        );
        who
    }

    /// A body that never moves.
    pub fn post(&mut self, x: u32, y: u32) -> Handle {
        let who = self.store.insert(());
        self.occupancy.place(self.map.index(x, y), who).unwrap();
        who
    }

    pub fn step(&mut self, orders: &[(Handle, MoveOrder)]) -> Vec<MovementEvent> {
        let mut events = Vec::new();
        // Stone everywhere, heard at 6 metres; gravel (9) on row 0 at column 1 and beyond.
        let noise = |cell: Cell| {
            if cell.0 >= 1 && cell.0 < 3 {
                Fixed32::from_int(9)
            } else {
                Fixed32::from_int(6)
            }
        };
        let mut world = MovementWorld {
            map: &self.map,
            occupancy: &mut self.occupancy,
            pathfinder: &mut self.pathfinder,
            rules: &self.rules,
            steps_per_second: self.rate,
            walking_noise: &noise,
        };
        step_movement(
            &mut self.movers,
            &mut self.fighters,
            &mut world,
            orders,
            &mut events,
        );
        events
    }

    pub fn wait(&mut self, steps: u32) -> Vec<MovementEvent> {
        (0..steps).flat_map(|_| self.step(&[])).collect()
    }

    pub fn at(&self, who: Handle) -> (u32, u32) {
        self.map.coordinates(self.occupancy.cell_of(who).unwrap())
    }

    pub fn stamina(&self, who: Handle) -> Fixed32 {
        self.fighters.get(who).unwrap().stamina
    }
}

pub fn moves(events: &[MovementEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, MovementEvent::Moved { .. }))
        .count()
}
