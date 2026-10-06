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

#[test]
fn rising_ground_stops_an_arrow_and_one_launched_from_a_hill_flies_over_a_low_wall() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let hill = duel.map.index(8, 2);
    duel.map.set_elevation(hill, 5);
    let events = fire(&mut duel, left, (5, 2), arrow(6, 1));
    let (at, reason) = stopped(&events).unwrap();
    assert_eq!(
        (duel.map.coordinates(at), reason),
        ((7, 2), Stopped::Wall),
        "the hill"
    );

    // From a cell 3 high, a 2-high wall at ground level sits well below the arrow.
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let perch = duel.map.index(2, 2);
    duel.map.set_elevation(perch, 3);
    let wall = duel.map.index(4, 2);
    duel.map.set_low_wall(wall, 2);
    let events = fire(&mut duel, left, (2, 2), arrow(4, 1));
    assert_eq!(stopped(&events).unwrap().1, Stopped::Spent);
}

#[test]
fn a_projectile_clears_a_low_wall_corner_it_flies_above_and_not_a_full_one() {
    // Heading north-east from (2, 3): the diagonal passes between (2, 2) and (3, 3).
    let shot = |full: bool| {
        let mut duel = Duel::new();
        let left = duel.left;
        duel.occupancy.vacate(duel.right);
        let side = duel.map.index(3, 3);
        if full {
            duel.map.set_passable(side, false);
        } else {
            duel.map.set_low_wall(side, 1);
        }
        let mut projectiles = StableVector::new();
        let from = duel.map.index(2, 3);
        let mut launch = arrow(1, 1);
        launch.height = 5;
        projectiles.insert(Projectile::launch(&duel.map, left, from, 8_192, &launch));
        let mut events = Vec::new();
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
        stopped(&events).unwrap()
    };
    assert_eq!(
        shot(false).1,
        Stopped::Spent,
        "the low corner is below the arrow"
    );
    assert_eq!(shot(true).1, Stopped::Wall, "a full wall corner stops it");
}

#[test]
fn a_projectile_passes_its_owner_and_hits_the_next_body() {
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    // Fired from behind its owner: the line crosses the owner's cell first.
    let events = fire(&mut duel, left, (3, 2), arrow(6, 1));
    let target = events.iter().find_map(|event| match event {
        CombatEvent::ProjectileHit { hit, .. } => Some(hit.target),
        _ => None,
    });
    assert_eq!(target, Some(right));
}

#[test]
fn a_dodging_body_two_cells_wide_dodges_an_arrow_once() {
    let mut duel = Duel::new();
    duel.dodge().distance_cells = 0;
    let (left, right) = (duel.left, duel.right);
    let wide = [duel.map.index(6, 2), duel.map.index(7, 2)];
    duel.occupancy.move_footprint(right, &wide).unwrap();
    duel.step(&[(right, Order::Dodge { heading: 0 })]);
    duel.wait(4);
    let events = fire(&mut duel, left, (5, 2), arrow(5, 1));
    let dodges = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                CombatEvent::Dodged { .. } | CombatEvent::PerfectDodge { .. }
            )
        })
        .count();
    assert_eq!(dodges, 1);
}

#[test]
fn a_projectile_reports_running_out_on_the_step_it_does() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.occupancy.vacate(duel.right);
    let mut projectiles = StableVector::new();
    let from = duel.map.index(5, 2);
    projectiles.insert(Projectile::launch(
        &duel.map,
        left,
        from,
        EAST,
        &arrow(4, 2),
    ));
    let mut steps = 0;
    while !projectiles.is_empty() {
        let mut events = Vec::new();
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
        steps += 1;
    }
    assert_eq!(steps, 2, "four cells at two a step");
}
