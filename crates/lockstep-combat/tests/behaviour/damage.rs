// SPDX-License-Identifier: Apache-2.0
use lockstep_combat::{resolve, DamageKind, DamagePacket, DamageResult, Defence, Tags};
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, SmoothedState, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn packet(amount: i32) -> DamagePacket {
    DamagePacket {
        amount: Fixed32::from_int(amount),
        kind: DamageKind::Blunt,
        knockback: 2,
        stagger: Fixed32::from_int(10),
        critical: Chance::NEVER,
        critical_multiplier: Fixed32::from_int(2),
        tags: Tags::NONE,
    }
}

fn hit(packet: &DamagePacket, defence: &Defence) -> DamageResult {
    resolve(
        packet,
        defence,
        &mut SmoothedState::default(),
        &mut Streams::new(7),
    )
}

#[test]
fn a_plain_hit_deals_its_amount_staggers_and_knocks_back() {
    let result = hit(&packet(40), &Defence::default());
    assert_eq!(result.dealt, Fixed32::from_int(40));
    assert!(!result.evaded && !result.blocked && !result.was_critical);
    assert!(result.staggered);
    assert_eq!(result.knockback, 2);
}

#[test]
fn certain_evasion_dodges_everything_but_an_unavoidable_hit() {
    let defence = Defence {
        evasion: Chance::ALWAYS,
        ..Defence::default()
    };
    let evaded = hit(&packet(40), &defence);
    assert!(evaded.evaded);
    assert_eq!(
        (evaded.dealt, evaded.knockback, evaded.staggered),
        (Fixed32::ZERO, 0, false)
    );
    let mut unavoidable = packet(40);
    unavoidable.tags = Tags::UNAVOIDABLE;
    assert_eq!(hit(&unavoidable, &defence).dealt, Fixed32::from_int(40));
}

#[test]
fn a_block_takes_its_fraction_and_stops_the_knockback() {
    let defence = Defence {
        blocking: true,
        block: Fixed32::from_ratio(3, 4),
        ..Defence::default()
    };
    let result = hit(&packet(40), &defence);
    assert!(result.blocked);
    assert_eq!((result.dealt, result.knockback), (Fixed32::from_int(10), 0));
}

#[test]
fn a_critical_multiplies_before_armour() {
    let mut critical = packet(40);
    critical.critical = Chance::ALWAYS;
    let defence = Defence {
        armour: Fixed32::from_int(10),
        ..Defence::default()
    };
    let result = hit(&critical, &defence);
    assert!(result.was_critical);
    assert_eq!(result.dealt, Fixed32::from_int(70), "40 × 2 − 10");
}

#[test]
fn armour_above_the_hit_floors_at_zero_and_poise_resists_stagger() {
    let defence = Defence {
        armour: Fixed32::from_int(100),
        poise: Fixed32::from_int(10),
        ..Defence::default()
    };
    let result = hit(&packet(40), &defence);
    assert_eq!(result.dealt, Fixed32::ZERO);
    assert!(!result.staggered, "stagger must exceed poise, not equal it");
}

#[test]
fn evasion_is_smoothed_per_target() {
    let defence = Defence {
        evasion: Chance::percent(50),
        ..Defence::default()
    };
    let mut memory = SmoothedState::default();
    let mut streams = Streams::new(3);
    let mut landed_in_a_row = 0;
    let mut evaded = 0;
    for _ in 0..2_000 {
        if resolve(&packet(1), &defence, &mut memory, &mut streams).evaded {
            evaded += 1;
            landed_in_a_row = 0;
        } else {
            landed_in_a_row += 1;
            assert!(
                landed_in_a_row < 4,
                "a 50 percent smoothed evasion never fails four times"
            );
        }
    }
    assert!((900..=1_100).contains(&evaded), "{evaded}");
}

#[test]
fn a_negative_resistance_is_a_weakness() {
    let mut defence = Defence::default();
    defence.resistances[DamageKind::Blunt.index()] = -Fixed32::HALF;
    assert_eq!(hit(&packet(40), &defence).dealt, Fixed32::from_int(60));
    defence.resistances[DamageKind::Blunt.index()] = Fixed32::from_int(-5);
    assert_eq!(
        hit(&packet(40), &defence).dealt,
        Fixed32::from_int(80),
        "held at double"
    );
}
