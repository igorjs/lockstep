// SPDX-License-Identifier: Apache-2.0
//! Decisions about commitment and dodging, each pinned by the number the spec gives:
//! - A stagger interrupts a wind-up whose action allows it, never a recovery.
//! - A hit during invulnerability is dodged; a grab lands through a dodge.
//! - A dodge pressed at most 0.12 seconds before the attack's first active step is perfect. At 30
//!   steps a second, 3 steps (0.100 seconds) is perfect and 4 steps (0.133 seconds) is not.
//! - An order pressed during recovery waits (up to 0.15 seconds) and fires on the first free step.
//!
//! Alternative rejected: interrupting at any phase (commitment would mean nothing), a perfect
//! window measured from invulnerability rather than the press, and dropping orders sent while busy.
//!
//! Would change if: any of the cases below changes outcome.

use crate::duel::{attack_order, Duel, EAST, GRAB, HEAVY, JAB, WEST};
use lockstep_combat::{CombatEvent, Order, Phase};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_stagger_interrupts_a_windup_but_not_a_recovery() {
    // Left starts a heavy blow; right starts a jab two steps later, so right is still winding up
    // when the heavy blow strikes: interrupted.
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    duel.step(&[(left, attack_order(HEAVY, EAST))]);
    duel.wait(1);
    duel.step(&[(right, attack_order(JAB, WEST))]);
    let events = duel.wait(4);
    assert!(events.contains(&CombatEvent::Interrupted {
        who: right,
        action: JAB,
        by: left
    }));
    assert_eq!(duel.fighter(right).phase, Phase::Ready);

    // Now right jabs and is in recovery when the heavy blow lands: no interruption.
    let mut duel = Duel::new();
    let (left, right) = (duel.left, duel.right);
    duel.step(&[(right, attack_order(JAB, WEST))]);
    duel.wait(9); // wind-up 6 steps, active 3: recovery begins on step 9
    assert!(matches!(duel.fighter(right).phase, Phase::Recovery { .. }));
    duel.step(&[(left, attack_order(HEAVY, EAST))]);
    let events = duel.wait(6);
    assert!(events
        .iter()
        .any(|event| matches!(event, CombatEvent::Landed { who, .. } if *who == left)));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CombatEvent::Interrupted { .. })));
}

/// Left attacks; right presses dodge `steps_before` steps before left's first active step.
fn dodge_before(action: lockstep_combat::ActionId, steps_before: u32) -> Vec<CombatEvent> {
    let mut duel = Duel::new();
    // Dodge in place, so only invulnerability decides the outcome.
    duel.dodge().distance_cells = 0;
    let (left, right) = (duel.left, duel.right);
    // Left's attack starts now; its first active step is 6 steps later.
    let mut events = duel.step(&[(left, attack_order(action, EAST))]);
    events.extend(duel.wait(5 - steps_before));
    events.extend(duel.step(&[(right, Order::Dodge { heading: 0 })]));
    events.extend(duel.wait(steps_before + 2));
    events
}

#[test]
fn a_hit_during_invulnerability_is_dodged_and_a_grab_lands_through_it() {
    let events = dodge_before(JAB, 4);
    assert!(events
        .iter()
        .any(|event| matches!(event, CombatEvent::Dodged { .. })));
    let events = dodge_before(GRAB, 4);
    assert!(!events.iter().any(|event| matches!(
        event,
        CombatEvent::Dodged { .. } | CombatEvent::PerfectDodge { .. }
    )));
    assert!(events
        .iter()
        .any(|event| matches!(event, CombatEvent::Landed { .. })));
}

#[test]
fn a_dodge_three_steps_before_the_strike_is_perfect_and_four_is_not() {
    let perfect = dodge_before(JAB, 3);
    assert!(
        perfect
            .iter()
            .any(|event| matches!(event, CombatEvent::PerfectDodge { .. })),
        "{perfect:?}"
    );
    let late = dodge_before(JAB, 4);
    assert!(
        late.iter()
            .any(|event| matches!(event, CombatEvent::Dodged { .. })),
        "{late:?}"
    );
    assert!(!late
        .iter()
        .any(|event| matches!(event, CombatEvent::PerfectDodge { .. })));
}

#[test]
fn a_dodge_pressed_during_recovery_fires_on_the_first_free_step() {
    let mut duel = Duel::new();
    let right = duel.right;
    duel.step(&[(right, attack_order(JAB, WEST))]);
    duel.wait(8);
    // Recovery is 9 steps; press dodge with 3 left, inside the 0.15 second buffer.
    duel.wait(6);
    let mut steps_until_dodge = 0;
    let mut events = duel.step(&[(right, Order::Dodge { heading: 0 })]);
    while !events.contains(&CombatEvent::DodgeStarted { who: right }) {
        steps_until_dodge += 1;
        assert!(steps_until_dodge < 10, "the dodge never fired");
        events = duel.step(&[]);
    }
    assert_eq!(steps_until_dodge, 3, "the first step after recovery ends");
}
