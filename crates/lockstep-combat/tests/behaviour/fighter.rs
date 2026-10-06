// SPDX-License-Identifier: Apache-2.0
use crate::duel::{attack_order, Duel, EAST, HEAVY, JAB, WEST};
use lockstep_combat::{ActionId, CombatEvent, Order, Phase, Refusal};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// The step (counted from the order) on which each event first appears.
fn timeline(
    duel: &mut Duel,
    first: &[(lockstep_core::Handle, Order)],
    steps: u32,
) -> Vec<(u32, CombatEvent)> {
    let mut seen = Vec::new();
    for event in duel.step(first) {
        seen.push((0, event));
    }
    for step in 1..=steps {
        for event in duel.step(&[]) {
            seen.push((step, event));
        }
    }
    seen
}

#[test]
fn an_attack_winds_up_strikes_once_and_recovers() {
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    let seen = timeline(&mut duel, &[(left, attack_order(JAB, EAST))], 20);
    assert_eq!(
        seen[0],
        (
            0,
            CombatEvent::Started {
                who: left,
                action: JAB
            }
        )
    );
    let landed: Vec<_> = seen
        .iter()
        .filter_map(|(step, event)| match event {
            CombatEvent::Landed { hits, .. } => {
                Some((*step, hits.len(), hits[0].target, hits[0].result.dealt))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        landed,
        [(6, 1, right, Fixed32::from_int(10))],
        "one strike, on the first active step"
    );
    assert_eq!(
        duel.fighter(left).phase,
        Phase::Ready,
        "free again after recovery"
    );
    assert_eq!(duel.fighter(left).stamina, Fixed32::from_int(90));
}

#[test]
fn an_attack_facing_away_whiffs() {
    let mut duel = Duel::new();
    let left = duel.left;
    let seen = timeline(&mut duel, &[(left, attack_order(JAB, WEST))], 7);
    assert!(seen.contains(&(
        6,
        CombatEvent::Whiffed {
            who: left,
            action: JAB
        }
    )));
}

#[test]
fn tired_fighters_and_unknown_actions_are_refused() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.fighters.get_mut(left).unwrap().stamina = Fixed32::from_int(5);
    let events = duel.step(&[(left, attack_order(JAB, EAST))]);
    assert_eq!(
        events,
        [CombatEvent::Refused {
            who: left,
            reason: Refusal::Tired
        }]
    );
    duel.fighters.get_mut(left).unwrap().stamina = Fixed32::from_int(100);
    let events = duel.step(&[(left, attack_order(ActionId(99), EAST))]);
    assert_eq!(
        events,
        [CombatEvent::Refused {
            who: left,
            reason: Refusal::UnknownAction
        }]
    );
}

#[test]
fn an_order_sent_while_busy_for_too_long_expires() {
    let mut duel = Duel::new();
    let left = duel.left;
    duel.step(&[(left, attack_order(JAB, EAST))]);
    // Early in the wind-up, with far more than 0.15 seconds of commitment left.
    let mut events = duel.step(&[(left, attack_order(JAB, EAST))]);
    events.extend(duel.wait(6));
    assert!(events.contains(&CombatEvent::Expired { who: left }));
    let started = events
        .iter()
        .filter(|event| matches!(event, CombatEvent::Started { .. }))
        .count();
    assert_eq!(started, 0, "the second jab never started");
}

#[test]
fn a_perfect_dodge_opens_a_counter_that_skips_the_windup() {
    let mut duel = Duel::new();
    duel.dodge().distance_cells = 0;
    let (left, right) = (duel.left, duel.right);
    duel.step(&[(left, attack_order(JAB, EAST))]);
    duel.wait(2);
    let mut events = duel.step(&[(right, Order::Dodge { heading: 0 })]);
    events.extend(duel.wait(3));
    assert!(events.contains(&CombatEvent::PerfectDodge {
        who: right,
        by: left
    }));
    assert_eq!(
        duel.fighter(right).stamina,
        Fixed32::from_int(100),
        "the dodge was refunded"
    );
    assert!(duel.fighter(right).countering());
    // Recover from the dodge; the window waits for the fighter to be free, then the counter lands
    // on the step it starts.
    duel.wait(20);
    assert_eq!(duel.fighter(right).phase, Phase::Ready);
    assert!(
        duel.fighter(right).countering(),
        "the window has not started running yet"
    );
    duel.wait(5);
    let events = duel.step(&[(right, attack_order(JAB, WEST))]);
    assert!(events.contains(&CombatEvent::Started {
        who: right,
        action: JAB
    }));
    assert!(events
        .iter()
        .any(|event| matches!(event, CombatEvent::Landed { who, .. } if *who == right)));
}

#[test]
fn a_dodge_moves_its_distance_and_a_wall_shortens_it() {
    let mut duel = Duel::new();
    let right = duel.right;
    duel.step(&[(right, Order::Dodge { heading: 0 })]);
    duel.wait(3);
    assert_eq!(
        duel.map.coordinates(duel.occupancy.cell_of(right).unwrap()),
        (8, 2)
    );
    assert!(duel.fighter(right).invulnerable());

    let mut duel = Duel::new();
    let right = duel.right;
    let wall = duel.map.index(7, 2);
    duel.map.set_passable(wall, false);
    duel.step(&[(right, Order::Dodge { heading: 0 })]);
    duel.wait(3);
    assert_eq!(
        duel.map.coordinates(duel.occupancy.cell_of(right).unwrap()),
        (6, 2),
        "blocked"
    );
    assert!(
        duel.fighter(right).invulnerable(),
        "a wall shortens the dodge, not its invulnerability"
    );
}

#[test]
fn a_second_dodge_waits_for_the_cooldown() {
    let mut duel = Duel::new();
    // A one second cooldown, far longer than the 17 steps a dodge takes.
    duel.dodge().cooldown_seconds = lockstep_core::math::Fixed32::ONE;
    let right = duel.right;
    duel.step(&[(right, Order::Dodge { heading: 0 })]);
    duel.wait(17);
    assert_eq!(
        duel.fighter(right).phase,
        Phase::Ready,
        "free, but still cooling down"
    );
    // Pressed 2 steps before the cooldown ends (step 30): it waits in the buffer, then fires.
    duel.wait(10);
    let mut at = 28;
    let mut events = duel.step(&[(right, Order::Dodge { heading: 0 })]);
    while !events.contains(&CombatEvent::DodgeStarted { who: right }) {
        at += 1;
        assert!(at < 40, "the second dodge never started");
        events = duel.step(&[]);
    }
    assert_eq!(at, 30, "the step the cooldown runs out");
}

#[test]
fn a_perfect_dodge_refunds_once_whoever_strikes() {
    let mut duel = Duel::new();
    duel.dodge().distance_cells = 0;
    let right = duel.right;
    // A third fighter on the other side of right, so two attackers strike right together.
    let mut store = lockstep_core::StableVector::new();
    store.insert(());
    store.insert(());
    let third = store.insert(());
    duel.occupancy.place(duel.map.index(7, 2), third).unwrap();
    duel.fighters.set(
        third,
        lockstep_combat::Fighter::new(
            lockstep_combat::MovesetId(0),
            Fixed32::from_int(100),
            Default::default(),
        ),
    );
    let left = duel.left;
    duel.step(&[
        (left, attack_order(JAB, EAST)),
        (third, attack_order(JAB, WEST)),
    ]);
    duel.wait(2);
    let mut events = duel.step(&[(right, Order::Dodge { heading: 0 })]);
    events.extend(duel.wait(3));
    let perfect = events
        .iter()
        .filter(|event| matches!(event, CombatEvent::PerfectDodge { .. }))
        .count();
    let dodged = events
        .iter()
        .filter(|event| matches!(event, CombatEvent::Dodged { .. }))
        .count();
    assert_eq!((perfect, dodged), (1, 1), "{events:?}");
    assert_eq!(
        duel.fighter(right).stamina,
        Fixed32::from_int(100),
        "refunded once, never above the maximum"
    );
}

#[test]
fn a_same_step_trade_of_heavy_blows_lands_for_both() {
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    duel.step(&[
        (left, attack_order(HEAVY, EAST)),
        (right, attack_order(HEAVY, WEST)),
    ]);
    let events = duel.wait(6);
    let landed: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            CombatEvent::Landed { who, .. } => Some(*who),
            _ => None,
        })
        .collect();
    assert_eq!(landed, [left, right]);
}

#[test]
fn phases_last_whole_steps_at_any_step_rate() {
    for (rate, windup_steps) in [(30, 6), (60, 12), (20, 4)] {
        let mut duel = Duel::new();
        duel.rate = rate;
        let left = duel.left;
        duel.step(&[(left, attack_order(JAB, EAST))]);
        let mut steps = 0;
        while !matches!(duel.fighter(left).phase, Phase::Active { .. }) {
            duel.step(&[]);
            steps += 1;
            assert!(steps < 100);
        }
        assert_eq!(
            steps, windup_steps,
            "a 0.2 second wind-up at {rate} steps a second"
        );
    }
}

#[test]
fn a_zero_length_recovery_costs_no_step() {
    let mut duel = Duel::new();
    duel.movesets[0].actions[0].recovery_seconds = Fixed32::ZERO;
    let left = duel.left;
    duel.step(&[(left, attack_order(JAB, EAST))]);
    duel.wait(9); // wind-up 6, active 3
    assert_eq!(duel.fighter(left).phase, Phase::Ready);
}

#[test]
fn a_stagger_breaks_a_dodge_in_its_startup() {
    let mut duel = Duel::new();
    // A longer startup, so the heavy blow lands while right is still starting its dodge.
    duel.dodge().startup_seconds = crate::duel::seconds(20);
    let (left, right) = (duel.left, duel.right);
    duel.step(&[(left, attack_order(HEAVY, EAST))]);
    duel.wait(3);
    let mut events = duel.step(&[(right, Order::Dodge { heading: 0 })]);
    events.extend(duel.wait(3));
    assert!(
        events.contains(&CombatEvent::DodgeBroken {
            who: right,
            by: left
        }),
        "{events:?}"
    );
    assert_eq!(duel.fighter(right).phase, Phase::Ready);
}

#[test]
fn a_dodge_cancels_a_windup() {
    let mut duel = Duel::new();
    let right = duel.right;
    duel.step(&[(right, attack_order(JAB, WEST))]);
    duel.wait(2);
    assert!(matches!(duel.fighter(right).phase, Phase::Windup { .. }));
    let events = duel.step(&[(right, Order::Dodge { heading: 0 })]);
    assert!(events.contains(&CombatEvent::DodgeStarted { who: right }));
    assert!(matches!(
        duel.fighter(right).phase,
        Phase::DodgeStartup { .. }
    ));
}

#[test]
fn a_heavy_blow_knocks_the_target_back_a_cell() {
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    let seen = timeline(&mut duel, &[(left, attack_order(HEAVY, EAST))], 7);
    let hit = seen
        .iter()
        .find_map(|(_, event)| match event {
            CombatEvent::Landed { hits, .. } => Some(hits[0].clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(hit.target, right);
    assert!(hit.result.staggered);
    assert_eq!(hit.knocked.unwrap().moved, 1);
    assert_eq!(
        duel.map.coordinates(duel.occupancy.cell_of(right).unwrap()),
        (7, 2)
    );
}

#[test]
fn the_same_orders_give_the_same_events() {
    let run = || {
        let mut duel = Duel::new();
        let (left, right) = (duel.left, duel.right);
        let mut all = duel.step(&[
            (left, attack_order(JAB, EAST)),
            (right, Order::Dodge { heading: 0 }),
        ]);
        all.extend(duel.wait(30));
        all.extend(duel.step(&[(right, attack_order(HEAVY, WEST))]));
        all.extend(duel.wait(30));
        (all, lockstep_core::hash_of(&duel.fighters))
    };
    assert_eq!(run(), run());
}
