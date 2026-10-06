// SPDX-License-Identifier: Apache-2.0
use crate::duel::{Duel, EAST};
use lockstep_combat::{
    step_projectiles, CombatEvent, DamageKind, DamagePacket, Launch, Order, Projectile, Stopped,
    Tags,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, Handle, StableVector};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn arrow(range: u8, speed: u8) -> Launch {
    Launch {
        range_cells: range,
        cells_per_step: speed,
        damage: Some(DamagePacket {
            amount: Fixed32::from_int(12),
            kind: DamageKind::Pierce,
            knockback: 0,
            stagger: Fixed32::ZERO,
            critical: Chance::NEVER,
            critical_multiplier: Fixed32::ONE,
            tags: Tags::NONE,
        }),
        loudness: 0,
        height: 2,
        descent_every: 0,
    }
}

/// Fires from `from` toward the east and steps until it stops; returns every event.
fn fire(duel: &mut Duel, owner: Handle, from: (u32, u32), launch: Launch) -> Vec<CombatEvent> {
    let mut projectiles = StableVector::new();
    let cell = duel.map.index(from.0, from.1);
    projectiles.insert(Projectile::launch(&duel.map, owner, cell, EAST, &launch));
    let mut events = Vec::new();
    for _ in 0..40 {
        if projectiles.is_empty() {
            break;
        }
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
    assert!(projectiles.is_empty(), "every projectile stops");
    events
}

fn stopped(events: &[CombatEvent]) -> Option<(lockstep_spatial::Cell, Stopped)> {
    events.iter().find_map(|event| match event {
        CombatEvent::ProjectileStopped { at, reason, .. } => Some((*at, *reason)),
        _ => None,
    })
}

#[test]
fn a_projectile_hits_the_first_body_that_is_not_its_owner() {
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    let events = fire(&mut duel, left, (5, 2), arrow(6, 2));
    let hit = events
        .iter()
        .find_map(|event| match event {
            CombatEvent::ProjectileHit { by, hit, .. } => Some((*by, hit.target, hit.result.dealt)),
            _ => None,
        })
        .unwrap();
    assert_eq!(hit, (left, right, Fixed32::from_int(12)));
}

#[test]
fn a_wall_stops_a_projectile_in_front_of_it() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let wall = duel.map.index(9, 2);
    duel.map.set_passable(wall, false);
    let events = fire(&mut duel, left, (5, 2), arrow(6, 3));
    let (at, reason) = stopped(&events).unwrap();
    assert_eq!((duel.map.coordinates(at), reason), ((8, 2), Stopped::Wall));
}

#[test]
fn a_low_wall_stops_a_low_projectile_and_a_high_one_flies_over() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let low_wall = duel.map.index(7, 2);
    duel.map.set_low_wall(low_wall, 2);
    let events = fire(&mut duel, left, (5, 2), arrow(4, 1));
    assert_eq!(
        stopped(&events).unwrap().1,
        Stopped::Wall,
        "height 2 against a wall of 2"
    );
    let mut high = arrow(4, 1);
    high.height = 3;
    let events = fire(&mut duel, left, (5, 2), high);
    assert_eq!(stopped(&events).unwrap().1, Stopped::Spent);
}

#[test]
fn a_projectile_spends_its_range_or_stops_at_the_edge() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let events = fire(&mut duel, left, (5, 2), arrow(3, 1));
    let (at, reason) = stopped(&events).unwrap();
    assert_eq!((duel.map.coordinates(at), reason), ((8, 2), Stopped::Spent));
    let events = fire(&mut duel, left, (5, 2), arrow(20, 2));
    let (at, reason) = stopped(&events).unwrap();
    assert_eq!(
        (duel.map.coordinates(at), reason),
        ((11, 2), Stopped::Wall),
        "the map edge"
    );
}

#[test]
fn a_descending_projectile_lands() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let mut lob = arrow(10, 1);
    lob.descent_every = 2;
    let events = fire(&mut duel, left, (1, 2), lob);
    let (at, reason) = stopped(&events).unwrap();
    assert_eq!(
        (duel.map.coordinates(at), reason),
        ((5, 2), Stopped::Landed),
        "height 2, one lost every 2 cells"
    );
}

#[test]
fn a_scream_damages_nobody_and_is_heard_along_its_way() {
    let mut duel = Duel::new();
    let left = duel.left;
    let scream = Launch {
        damage: None,
        loudness: 8,
        ..arrow(4, 2)
    };
    let events = fire(&mut duel, left, (5, 2), scream);
    assert!(!events
        .iter()
        .any(|event| matches!(event, CombatEvent::ProjectileHit { .. })));
    let heard: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            CombatEvent::Sounded { at, loudness, .. } => {
                Some((duel.map.coordinates(*at), *loudness))
            }
            _ => None,
        })
        .collect();
    assert_eq!(heard, [((6, 2), 8), ((7, 2), 8), ((8, 2), 8), ((9, 2), 8)]);
    assert_eq!(stopped(&events).unwrap().1, Stopped::Spent);
}

#[test]
fn a_dodging_fighter_lets_a_projectile_pass() {
    let mut duel = Duel::new();
    duel.dodge().distance_cells = 0;
    let (left, right) = (duel.left, duel.right);
    duel.step(&[(right, Order::Dodge { heading: 0 })]);
    duel.wait(2);
    assert!(duel.fighter(right).invulnerable());
    let events = fire(&mut duel, left, (5, 2), arrow(4, 1));
    assert!(events.iter().any(|event| matches!(
        event,
        CombatEvent::Dodged { .. } | CombatEvent::PerfectDodge { .. }
    )));
    assert_eq!(
        stopped(&events).unwrap().1,
        Stopped::Spent,
        "it flew on past"
    );
}
