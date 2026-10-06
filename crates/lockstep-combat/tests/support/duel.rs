// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]
//! Two fighters facing each other on a small map, for the action and dodge tests.

use lockstep_combat::{
    step_combat, ActionDefinition, ActionId, CombatEvent, DamageKind, DamagePacket, Defence,
    DodgeDefinition, Fighter, HitShape, InterruptMask, Moveset, MovesetId, Order, Tags,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, Column, Handle, StableVector, Streams};
use lockstep_spatial::{GridMap, Occupancy, Square8};

pub const JAB: ActionId = ActionId(0);
pub const GRAB: ActionId = ActionId(1);
pub const HEAVY: ActionId = ActionId(2);
pub const EAST: u16 = 0;
pub const WEST: u16 = 32_768;
pub const NORTH: u16 = 16_384;

pub fn seconds(hundredths: i32) -> Fixed32 {
    Fixed32::from_ratio(hundredths, 100)
}

fn attack(windup: i32, damage: DamagePacket, interruptible_by: InterruptMask) -> ActionDefinition {
    ActionDefinition {
        windup_seconds: seconds(windup),
        active_seconds: seconds(10),
        recovery_seconds: seconds(30),
        shape: HitShape::Adjacent,
        stamina_cost: Fixed32::from_int(10),
        damage,
        interruptible_by,
    }
}

fn packet(amount: i32, stagger: i32, knockback: u8, tags: Tags) -> DamagePacket {
    DamagePacket {
        amount: Fixed32::from_int(amount),
        kind: DamageKind::Blunt,
        knockback,
        stagger: Fixed32::from_int(stagger),
        critical: Chance::NEVER,
        critical_multiplier: Fixed32::ONE,
        tags,
    }
}

pub fn moveset() -> Moveset {
    Moveset {
        actions: vec![
            // A jab: 0.2 s wind-up that a stagger interrupts, light stagger.
            attack(20, packet(10, 5, 0, Tags::NONE), InterruptMask::STAGGER),
            // A grab: pierces a dodge.
            attack(20, packet(5, 0, 0, Tags::GRAB), InterruptMask::STAGGER),
            // A heavy blow: staggers anything with poise under 50 and knocks back a cell.
            attack(20, packet(30, 50, 1, Tags::NONE), InterruptMask::NONE),
        ],
        dodge: DodgeDefinition {
            startup_seconds: seconds(5),
            invulnerable_seconds: seconds(30),
            recovery_seconds: seconds(20),
            distance_cells: 2,
            stamina_cost: Fixed32::from_int(20),
            cooldown_seconds: seconds(50),
            perfect_window_seconds: seconds(12),
            counter_seconds: seconds(50),
        },
    }
}

pub struct Duel {
    pub map: GridMap<Square8>,
    pub occupancy: Occupancy,
    pub fighters: Column<Fighter>,
    pub movesets: Vec<Moveset>,
    /// Steps a second.
    pub rate: u32,
    pub streams: Streams,
    /// Facing east, on the left.
    pub left: Handle,
    /// Facing west, one cell to the right.
    pub right: Handle,
}

impl Duel {
    pub fn new() -> Self {
        let map: GridMap<Square8> = GridMap::new(12, 5);
        let mut occupancy = Occupancy::new(&map);
        let mut store = StableVector::new();
        let (left, right) = (store.insert(()), store.insert(()));
        occupancy.place(map.index(5, 2), left).unwrap();
        occupancy.place(map.index(6, 2), right).unwrap();
        let mut fighters = Column::new();
        for who in [left, right] {
            fighters.set(
                who,
                Fighter::new(MovesetId(0), Fixed32::from_int(100), Defence::default()),
            );
        }
        Duel {
            map,
            occupancy,
            fighters,
            movesets: vec![moveset()],
            rate: 30,
            streams: Streams::new(5),
            left,
            right,
        }
    }

    /// One step at `rate` a second with these orders.
    pub fn step(&mut self, orders: &[(Handle, Order)]) -> Vec<CombatEvent> {
        let mut events = Vec::new();
        step_combat(
            &mut self.fighters,
            &self.movesets,
            &self.map,
            &mut self.occupancy,
            orders,
            self.rate,
            &mut self.streams,
            &mut events,
        );
        events
    }

    /// `count` quiet steps, with every event.
    pub fn wait(&mut self, count: u32) -> Vec<CombatEvent> {
        (0..count).flat_map(|_| self.step(&[])).collect()
    }

    pub fn fighter(&self, who: Handle) -> &Fighter {
        self.fighters.get(who).unwrap()
    }
}

pub fn attack_order(action: ActionId, facing: u16) -> Order {
    Order::Attack { action, facing }
}

impl Duel {
    pub fn dodge(&mut self) -> &mut DodgeDefinition {
        &mut self.movesets[0].dodge
    }
}
