// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{hash_of, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn adding_a_new_named_stream_does_not_change_any_existing_streams_sequence() {
    let mut alone = Streams::new(99);
    let alone_draws: Vec<i32> = (0..1000)
        .map(|_| alone.range("loot", 0, 1_000_000))
        .collect();

    let mut crowded = Streams::new(99);
    let mut crowded_draws = Vec::new();
    for index in 0..1000 {
        if index % 3 == 0 {
            crowded.range("weather", 0, 10);
        }
        crowded_draws.push(crowded.range("loot", 0, 1_000_000));
        if index % 5 == 0 {
            crowded.unit("encounters");
        }
    }
    assert_eq!(alone_draws, crowded_draws);
}

#[test]
fn different_seeds_and_different_names_give_different_sequences() {
    let mut first = Streams::new(1);
    let mut second = Streams::new(2);
    assert_ne!(
        first.range("loot", 0, i32::MAX),
        second.range("loot", 0, i32::MAX)
    );
    let mut third = Streams::new(1);
    assert_ne!(
        third.range("loot", 0, i32::MAX),
        third.range("combat", 0, i32::MAX)
    );
}

#[test]
fn ranges_and_units_stay_inside_their_bounds() {
    let mut streams = Streams::new(4);
    for _ in 0..10_000 {
        let value = streams.range("range", -5, 5);
        assert!((-5..5).contains(&value));
        let unit = streams.unit("unit");
        assert!((0.0..1.0).contains(&unit));
        assert!(streams.pick("pick", 7) < 7);
    }
    assert!(!streams.chance("never", 0.0));
}

#[test]
fn a_restored_stream_set_continues_the_same_sequences() {
    let mut original = Streams::new(11);
    for _ in 0..100 {
        original.range("loot", 0, 100);
        original.unit("weather");
    }
    let bytes = bincode::serialize(&original).unwrap();
    let mut restored: Streams = bincode::deserialize(&bytes).unwrap();
    assert_eq!(hash_of(&restored), hash_of(&original));
    for _ in 0..100 {
        assert_eq!(
            restored.range("loot", 0, 100),
            original.range("loot", 0, 100)
        );
        assert_eq!(restored.unit("weather"), original.unit("weather"));
    }
}

#[test]
fn the_same_million_rolls_hash_to_the_committed_value() {
    let committed = include_str!("../../fixtures/million_rolls.hash").trim();
    assert_eq!(format!("{:016x}", million_rolls_hash()), committed);
}

fn million_rolls_hash() -> u64 {
    let mut streams = Streams::new(20_260_925);
    let mut rolls: Vec<u16> = Vec::with_capacity(1_000_000);
    let mut units: Vec<u32> = Vec::new();
    for index in 0..1_000_000u32 {
        rolls.push(streams.range("loot", 0, 1000) as u16);
        if index % 1000 == 0 {
            units.push(streams.unit("weather").to_bits());
        }
    }
    hash_of(&(rolls, units))
}

/// Prints the value to commit. Run with `cargo test -p lockstep-core print_million_rolls_hash -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_million_rolls_hash() {
    println!("{:016x}", million_rolls_hash());
}
