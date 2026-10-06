// SPDX-License-Identifier: Apache-2.0
use crate::walkers::{moves, Walkers};
use lockstep_combat::{quantise_facing, Gait, MoveOrder, MovementEvent};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn order(walkers: &Walkers, x: u32, y: u32, gait: Gait) -> MoveOrder {
    MoveOrder::MoveTo {
        target: walkers.map.index(x, y),
        gait,
    }
}

#[test]
fn walking_covers_three_point_six_cells_a_second_and_running_eight_point_four() {
    for (gait, cells) in [(Gait::Walk, 36), (Gait::Run, 84), (Gait::Sneak, 18)] {
        let mut walkers = Walkers::new(200, 3);
        walkers.rules.run_drain_per_second = Fixed32::ZERO;
        let who = walkers.add(1, 1);
        let to = order(&walkers, 190, 1, gait);
        let mut events = walkers.step(&[(who, to)]);
        events.extend(walkers.wait(299));
        // Ten seconds, plus the first cell taken up front. Speeds are 16.16 fixed point, so 8.4
        // is a hair under and the last cell may land one step later.
        let moved = moves(&events);
        assert!(moved == cells + 1 || moved == cells, "{gait:?}: {moved}");
    }
}

#[test]
fn a_diagonal_step_costs_more_than_a_straight_one() {
    let mut straight = Walkers::new(60, 60);
    let a = straight.add(1, 1);
    let to = order(&straight, 50, 1, Gait::Walk);
    let mut events = straight.step(&[(a, to)]);
    events.extend(straight.wait(299));
    let mut diagonal = Walkers::new(60, 60);
    let b = diagonal.add(1, 1);
    let to = order(&diagonal, 50, 50, Gait::Walk);
    let mut diagonal_events = diagonal.step(&[(b, to)]);
    diagonal_events.extend(diagonal.wait(299));
    assert_eq!(moves(&events), 37);
    assert_eq!(moves(&diagonal_events), 26, "36 / 1.4, plus the first cell");
}

#[test]
fn a_held_move_order_changes_target_without_losing_progress() {
    let mut held = Walkers::new(60, 5);
    let who = held.add(1, 2);
    let mut moved = 0;
    for step in 0..60 {
        // The target moves every step, the way a held stick or button resends it.
        let target_x = 30 + step % 3;
        moved += moves(&held.step(&[(who, order(&held, target_x, 2, Gait::Walk))]));
    }
    let mut once = Walkers::new(60, 5);
    let same = once.add(1, 2);
    let mut moved_once = moves(&once.step(&[(same, order(&once, 30, 2, Gait::Walk))]));
    moved_once += moves(&once.wait(59));
    assert_eq!(moved, moved_once);
}

#[test]
fn a_body_in_the_way_is_slid_around() {
    let mut walkers = Walkers::new(20, 5);
    let who = walkers.add(2, 2);
    walkers.post(3, 2);
    let mut events = walkers.step(&[(who, order(&walkers, 10, 2, Gait::Walk))]);
    let mut steps = 0;
    while !events.contains(&MovementEvent::Arrived {
        who,
        at: walkers.map.index(10, 2),
    }) {
        assert!(steps < 120, "never arrived");
        assert!(
            !events.contains(&MovementEvent::Repathed { who }),
            "it slid, it did not detour"
        );
        events = walkers.step(&[]);
        steps += 1;
    }
    // Eight cells, two of them diagonal around the post: 6 × 1 + 2 × 1.4 = 8.8 cells at 3.6 a
    // second, about 74 steps after the first cell.
    assert!(steps < 80, "{steps} steps");
}

#[test]
fn a_body_blocked_for_half_a_second_finds_a_new_path() {
    // A one-cell corridor along row 1, blocked by a post; the long way round is open.
    let mut walkers = Walkers::new(12, 5);
    for x in 0..12 {
        if x != 0 && x != 11 {
            let wall = walkers.map.index(x, 2);
            walkers.map.set_passable(wall, false);
            let roof = walkers.map.index(x, 0);
            walkers.map.set_passable(roof, false);
        }
    }
    let who = walkers.add(1, 1);
    walkers.post(4, 1);
    let mut events = walkers.step(&[(who, order(&walkers, 9, 1, Gait::Walk))]);
    events.extend(walkers.wait(400));
    assert!(
        events.contains(&MovementEvent::Repathed { who }),
        "{events:?}"
    );
    assert!(events.contains(&MovementEvent::Arrived {
        who,
        at: walkers.map.index(9, 1)
    }));
}

#[test]
fn an_exhausted_runner_walks_until_stamina_is_back_at_fifteen() {
    let mut walkers = Walkers::new(250, 3);
    let who = walkers.add(1, 1);
    let to = order(&walkers, 248, 1, Gait::Run);
    walkers.step(&[(who, to)]);
    let mut events = Vec::new();
    let mut guard = 0;
    while !events.contains(&MovementEvent::Exhausted { who }) {
        events = walkers.step(&[]);
        guard += 1;
        assert!(guard < 1_000, "never ran out of stamina");
    }
    // Walking restores 4 a second: 15 takes 3.75 seconds, about 113 steps, at walking pace.
    let after = walkers.wait(100);
    assert_eq!(
        moves(&after),
        12,
        "walking pace: 3.6 cells a second for 100 steps"
    );
    assert!(!after.contains(&MovementEvent::Recovered { who }));
    let mut recovered_with = None;
    for _ in 0..20 {
        if walkers
            .step(&[])
            .contains(&MovementEvent::Recovered { who })
        {
            recovered_with = Some(walkers.stamina(who));
            break;
        }
    }
    let stamina = recovered_with.expect("recovered within 120 steps");
    assert!(
        stamina >= Fixed32::from_int(15) && stamina < Fixed32::from_int(16),
        "{stamina:?}"
    );
}

#[test]
fn facing_turns_to_the_nearest_of_eight_directions() {
    assert_eq!(quantise_facing(0), 0);
    assert_eq!(quantise_facing(4_095), 0);
    assert_eq!(quantise_facing(4_096), 8_192);
    assert_eq!(
        quantise_facing(65_000),
        0,
        "just under a full turn rounds to east"
    );
    let mut walkers = Walkers::new(20, 20);
    let who = walkers.add(5, 5);
    walkers.step(&[(who, order(&walkers, 9, 1, Gait::Walk))]);
    assert_eq!(walkers.movers.get(who).unwrap().facing, 8_192, "north-east");
}

#[test]
fn an_unreachable_target_is_reported_and_nothing_moves() {
    let mut walkers = Walkers::new(10, 3);
    for y in 0..3 {
        let wall = walkers.map.index(5, y);
        walkers.map.set_passable(wall, false);
    }
    let who = walkers.add(1, 1);
    let events = walkers.step(&[(who, order(&walkers, 8, 1, Gait::Walk))]);
    assert_eq!(events, [MovementEvent::Unreachable { who }]);
    assert_eq!(walkers.at(who), (1, 1));
}

#[test]
fn stop_halts_a_walk() {
    let mut walkers = Walkers::new(20, 3);
    let who = walkers.add(1, 1);
    walkers.step(&[(who, order(&walkers, 18, 1, Gait::Walk))]);
    walkers.wait(20);
    let before = walkers.at(who);
    walkers.step(&[(who, MoveOrder::Stop)]);
    assert_eq!(moves(&walkers.wait(60)), 0);
    assert_eq!(walkers.at(who), before);
}

#[test]
fn a_target_another_body_holds_is_approached_and_the_walker_halts_beside_it() {
    let mut walkers = Walkers::new(20, 9);
    let who = walkers.add(4, 5);
    walkers.post(8, 5);
    let mut events = walkers.step(&[(who, order(&walkers, 8, 5, Gait::Walk))]);
    events.extend(walkers.wait(200));
    assert!(
        events.contains(&MovementEvent::Halted {
            who,
            at: walkers.map.index(7, 5)
        }),
        "{events:?}"
    );
    assert_eq!(walkers.at(who), (7, 5));
    assert!(!walkers.movers.get(who).unwrap().moving());
}

#[test]
fn a_wall_raised_on_the_path_is_walked_around() {
    let mut walkers = Walkers::new(20, 5);
    let who = walkers.add(1, 2);
    walkers.step(&[(who, order(&walkers, 12, 2, Gait::Walk))]);
    walkers.wait(10);
    let door = walkers.map.index(8, 2);
    walkers.map.set_passable(door, false);
    let events = walkers.wait(200);
    assert!(!events
        .iter()
        .any(|event| matches!(event, MovementEvent::Moved { to, .. } if *to == door)));
    assert!(events.contains(&MovementEvent::Arrived {
        who,
        at: walkers.map.index(12, 2)
    }));
}

#[test]
fn a_held_target_that_keeps_changing_still_finds_a_way_around_a_blocker() {
    let mut walkers = Walkers::new(12, 5);
    for x in 1..11 {
        let wall = walkers.map.index(x, 2);
        walkers.map.set_passable(wall, false);
        let roof = walkers.map.index(x, 0);
        walkers.map.set_passable(roof, false);
    }
    let who = walkers.add(1, 1);
    walkers.post(4, 1);
    let mut repathed = false;
    for step in 0..120 {
        let target = 8 + step % 2;
        repathed |= walkers
            .step(&[(who, order(&walkers, target, 1, Gait::Walk))])
            .contains(&MovementEvent::Repathed { who });
    }
    assert!(
        repathed,
        "half a second blocked, even with the target changing every step"
    );
}

#[test]
fn holding_the_order_at_the_destination_arrives_once() {
    let mut walkers = Walkers::new(10, 3);
    let who = walkers.add(1, 1);
    let mut arrivals = 0;
    for _ in 0..90 {
        let events = walkers.step(&[(who, order(&walkers, 3, 1, Gait::Walk))]);
        arrivals += events
            .iter()
            .filter(|event| matches!(event, MovementEvent::Arrived { .. }))
            .count();
    }
    assert_eq!(arrivals, 1);
}

#[test]
fn each_surface_is_heard_at_its_own_distance() {
    let mut walkers = Walkers::new(10, 3);
    let who = walkers.add(0, 0);
    let events = walkers.step(&[(who, order(&walkers, 1, 0, Gait::Walk))]);
    let heard = events.iter().find_map(|event| match event {
        MovementEvent::Moved { noise_metres, .. } => Some(*noise_metres),
        _ => None,
    });
    assert_eq!(heard, Some(Fixed32::from_int(9)), "gravel");
}

#[test]
fn a_runner_held_behind_a_body_recovers_stamina() {
    // A sealed corridor: the post never moves and there is no way round.
    let mut walkers = Walkers::new(12, 3);
    for x in 0..12 {
        for y in [0, 2] {
            let wall = walkers.map.index(x, y);
            walkers.map.set_passable(wall, false);
        }
    }
    let who = walkers.add(1, 1);
    walkers.post(3, 1);
    walkers.fighters.get_mut(who).unwrap().stamina = Fixed32::from_int(50);
    walkers.step(&[(who, order(&walkers, 9, 1, Gait::Run))]);
    walkers.wait(60);
    assert!(
        walkers.stamina(who) > Fixed32::from_int(50),
        "{:?}",
        walkers.stamina(who)
    );
}
