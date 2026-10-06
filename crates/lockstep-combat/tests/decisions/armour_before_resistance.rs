// SPDX-License-Identifier: Apache-2.0
//! Decision: damage resolves in a fixed order (evasion, block, critical, flat armour, percentage
//! resistance, floor at zero, stagger, knockback), so flat armour comes off before resistance
//! scales what is left.
//! Alternative rejected: resistance first, which makes armour worth more against big hits and lets
//! a high resistance plus a little armour cancel any hit.
//! Would change if: 100 damage against 30 armour and 50 percent resistance deals anything but 35.

use lockstep_combat::{resolve, DamageKind, DamagePacket, Defence, Tags};
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, SmoothedState, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_hundred_against_thirty_armour_and_half_resistance_deals_thirty_five_not_twenty() {
    let packet = DamagePacket {
        amount: Fixed32::from_int(100),
        kind: DamageKind::Cut,
        knockback: 0,
        stagger: Fixed32::ZERO,
        critical: Chance::NEVER,
        critical_multiplier: Fixed32::ONE,
        tags: Tags::NONE,
    };
    let mut defence = Defence {
        armour: Fixed32::from_int(30),
        ..Defence::default()
    };
    defence.resistances[DamageKind::Cut.index()] = Fixed32::HALF;
    let result = resolve(
        &packet,
        &defence,
        &mut SmoothedState::default(),
        &mut Streams::new(1),
    );
    assert_eq!(result.dealt, Fixed32::from_int(35));
}
