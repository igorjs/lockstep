// SPDX-License-Identifier: Apache-2.0
//! Decision: a projectile flies at an absolute altitude (its launch ground plus its height); a
//! cell whose ground plus wall reaches that altitude stops it, so a wall exactly its height stops
//! it and rising ground does too. A dodged projectile flies on: a dodge moves out of its way, it
//! does not destroy it.
//! Alternative rejected: comparing the projectile's height with the wall alone, which lets arrows
//! fly through hills; and removing a projectile when it is dodged, which makes a dodge a shield.
//! Would change if: a projectile of height 2 passes a wall of height 2, or a dodged projectile
//! stops at the dodger.

use crate::duel::{Duel, EAST};
use lockstep_combat::{
    step_projectiles, CombatEvent, DamageKind, DamagePacket, Launch, Order, Projectile, Stopped,
    Tags,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn launch(height: u8) -> Launch {
    Launch {
        range_cells: 4,
        cells_per_step: 1,
        damage: Some(DamagePacket {
            amount: Fixed32::from_int(10),
            kind: DamageKind::Pierce,
            knockback: 0,
            stagger: Fixed32::ZERO,
            critical: Chance::NEVER,
            critical_multiplier: Fixed32::ONE,
            tags: Tags::NONE,
        }),
        loudness: 0,
        height,
        descent_every: 0,
    }
}

fn fly(duel: &mut Duel, height: u8) -> Vec<CombatEvent> {
    let mut projectiles = StableVector::new();
    let from = duel.map.index(5, 2);
    projectiles.insert(Projectile::launch(
        &duel.map,
        duel.left,
        from,
        EAST,
        &launch(height),
    ));
    let mut events = Vec::new();
    for _ in 0..10 {
        step_projectiles(
            &mut projectiles,
            &mut duel.fighters,
            &duel.movesets,
            &duel.map,
            &mut duel.occupancy,
            duel.rate,
            &mut duel.streams,
            &mut events,
        );
    }
    events
}

fn reason(events: &[CombatEvent]) -> Option<Stopped> {
    events.iter().find_map(|event| match event {
        CombatEvent::ProjectileStopped { reason, .. } => Some(*reason),
        _ => None,
    })
}

#[test]
fn a_wall_exactly_its_height_stops_a_projectile_and_one_lower_does_not() {
    for (height, expected) in [(2, Stopped::Wall), (3, Stopped::Spent)] {
        let mut duel = Duel::new();
        duel.occupancy.vacate(duel.right);
        let wall = duel.map.index(7, 2);
        duel.map.set_low_wall(wall, 2);
        assert_eq!(
            reason(&fly(&mut duel, height)),
            Some(expected),
            "height {height}"
        );
    }
}

#[test]
fn a_dodged_projectile_flies_on() {
    let mut duel = Duel::new();
    duel.dodge().distance_cells = 0;
    let right = duel.right;
    duel.step(&[(right, Order::Dodge { heading: 0 })]);
    duel.wait(3);
    let events = fly(&mut duel, 2);
    assert!(events.iter().any(|event| matches!(
        event,
        CombatEvent::Dodged { .. } | CombatEvent::PerfectDodge { .. }
    )));
    assert_eq!(
        reason(&events),
        Some(Stopped::Spent),
        "it went on past the dodger"
    );
}
