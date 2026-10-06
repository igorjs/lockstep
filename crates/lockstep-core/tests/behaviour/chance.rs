// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{Chance, SmoothedState, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn rate(successes: u32, rolls: u32) -> f64 {
    successes as f64 / rolls as f64
}

#[test]
fn a_chance_is_clamped_to_one_hundred_percent() {
    assert_eq!(Chance::basis_points(25_000), Chance::ALWAYS);
    assert_eq!(Chance::percent(150), Chance::ALWAYS);
    assert_eq!(Chance::percent(25), Chance(2_500));
    let mut streams = Streams::new(1);
    let mut state = SmoothedState::default();
    for _ in 0..10_000 {
        assert!(streams.roll("out of range", Chance(u32::MAX)));
        assert!(streams.roll_smoothed("out of range", Chance(u32::MAX), &mut state));
    }
}

#[test]
fn never_never_succeeds_and_always_always_does() {
    let mut streams = Streams::new(2);
    let mut never = SmoothedState::default();
    let mut always = SmoothedState::default();
    for _ in 0..20_000 {
        assert!(!streams.roll("plain", Chance::NEVER));
        assert!(streams.roll("plain", Chance::ALWAYS));
        assert!(!streams.roll_with_luck("lucky", Chance::NEVER, 5));
        assert!(streams.roll_with_luck("unlucky", Chance::ALWAYS, -5));
        assert!(!streams.roll_smoothed("smoothed", Chance::NEVER, &mut never));
        assert!(streams.roll_smoothed("smoothed", Chance::ALWAYS, &mut always));
    }
}

#[test]
fn a_plain_roll_lands_on_its_nominal_rate() {
    let mut streams = Streams::new(3);
    let successes = (0..200_000)
        .filter(|_| streams.roll("loot", Chance::percent(25)))
        .count();
    assert!((rate(successes as u32, 200_000) - 0.25).abs() < 0.005);
}

#[test]
fn luck_three_at_twenty_five_percent_lands_between_sixty_six_and_seventy_percent() {
    let mut streams = Streams::new(4);
    let lucky = (0..200_000)
        .filter(|_| streams.roll_with_luck("loot", Chance::percent(25), 3))
        .count();
    let lucky = rate(lucky as u32, 200_000);
    assert!((0.66..=0.70).contains(&lucky), "lucky rate {lucky}");
    // Four draws keeping the worst succeed only when all four do: 0.25^4 is about 0.4 percent.
    let unlucky = (0..200_000)
        .filter(|_| streams.roll_with_luck("loot", Chance::percent(25), -3))
        .count();
    let unlucky = rate(unlucky as u32, 200_000);
    assert!((0.002..=0.006).contains(&unlucky), "unlucky rate {unlucky}");
}

#[test]
fn luck_zero_is_a_plain_roll() {
    let mut plain = Streams::new(5);
    let mut lucky = Streams::new(5);
    for _ in 0..10_000 {
        assert_eq!(
            plain.roll("loot", Chance(3_333)),
            lucky.roll_with_luck("loot", Chance(3_333), 0)
        );
    }
}

#[test]
fn smoothed_seventy_percent_never_misses_five_in_a_row_and_converges_within_half_a_percent() {
    let mut streams = Streams::new(6);
    let mut state = SmoothedState::default();
    let (mut successes, mut misses_in_a_row) = (0, 0);
    for _ in 0..200_000 {
        if streams.roll_smoothed("critical", Chance::percent(70), &mut state) {
            successes += 1;
            misses_in_a_row = 0;
        } else {
            misses_in_a_row += 1;
            assert!(misses_in_a_row < 5);
        }
    }
    assert!((rate(successes, 200_000) - 0.70).abs() < 0.005);
}

#[test]
fn smoothed_rolls_converge_on_their_nominal_rate_with_shorter_droughts_than_plain_rolls() {
    for percent in [1, 5, 10, 25, 50, 90] {
        let mut streams = Streams::new(7 + percent as u64);
        let mut state = SmoothedState::default();
        let rolls = 400_000;
        let (mut smoothed, mut plain) = (0u32, 0u32);
        let (mut smoothed_drought, mut plain_drought) = (0u32, 0u32);
        let (mut smoothed_run, mut plain_run) = (0u32, 0u32);
        for _ in 0..rolls {
            if streams.roll_smoothed("smoothed", Chance::percent(percent), &mut state) {
                smoothed += 1;
                smoothed_run = 0;
            } else {
                smoothed_run += 1;
                smoothed_drought = smoothed_drought.max(smoothed_run);
            }
            if streams.roll("plain", Chance::percent(percent)) {
                plain += 1;
                plain_run = 0;
            } else {
                plain_run += 1;
                plain_drought = plain_drought.max(plain_run);
            }
        }
        let nominal = percent as f64 / 100.0;
        assert!(
            (rate(smoothed, rolls) - nominal).abs() < 0.005,
            "{percent} percent: smoothed rate {}",
            rate(smoothed, rolls)
        );
        assert!((rate(plain, rolls) - nominal).abs() < 0.005);
        assert!(
            smoothed_drought < plain_drought,
            "{percent} percent: drought {smoothed_drought} against {plain_drought}"
        );
        assert_eq!(
            state.failures, smoothed_run,
            "the state counts the current run"
        );
    }
}
