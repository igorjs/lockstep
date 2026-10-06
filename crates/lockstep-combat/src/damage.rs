// SPDX-License-Identifier: Apache-2.0
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, Message, SmoothedState, Streams};
use serde::{Deserialize, Serialize};

/// What a hit is made of. Each kind has its own resistance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DamageKind {
    Blunt,
    Cut,
    Pierce,
    Heat,
    Cold,
    Toxic,
}

impl DamageKind {
    pub const COUNT: usize = 6;

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Flags that change how a hit resolves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tags(u8);

impl Tags {
    pub const NONE: Tags = Tags(0);
    /// Skips evasion.
    pub const UNAVOIDABLE: Tags = Tags(1);
    /// A grab: lands through a dodge (see the dodge rules).
    pub const GRAB: Tags = Tags(2);

    pub fn contains(self, other: Tags) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn with(self, other: Tags) -> Tags {
        Tags(self.0 | other.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct DamagePacket {
    pub amount: Fixed32,
    pub kind: DamageKind,
    /// Cells to push the target when the hit lands unblocked.
    pub knockback: u8,
    /// Compared with the target's poise.
    pub stagger: Fixed32,
    pub critical: Chance,
    pub critical_multiplier: Fixed32,
    pub tags: Tags,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Defence {
    /// Subtracted from every hit.
    pub armour: Fixed32,
    /// The fraction of each kind taken away after armour: 0.5 halves it.
    pub resistances: [Fixed32; DamageKind::COUNT],
    /// A hit staggers when its stagger is greater than this.
    pub poise: Fixed32,
    /// The chance to evade, rolled smoothed so evasion is not streaky.
    pub evasion: Chance,
    /// Whether the target is blocking, and the fraction a block takes away.
    pub blocking: bool,
    pub block: Fixed32,
}

impl Default for Defence {
    fn default() -> Self {
        Defence {
            armour: Fixed32::ZERO,
            resistances: [Fixed32::ZERO; DamageKind::COUNT],
            poise: Fixed32::ZERO,
            evasion: Chance::NEVER,
            blocking: false,
            block: Fixed32::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageResult {
    pub dealt: Fixed32,
    pub evaded: bool,
    pub blocked: bool,
    pub was_critical: bool,
    pub staggered: bool,
    /// Cells the target is pushed; `knock_back` moves it.
    pub knockback: u8,
}

/// Resolves one hit, in a fixed order: evasion (a smoothed roll on the `"evasion"` stream,
/// skipped by `UNAVOIDABLE`), block, critical (on the `"combat"` stream), flat armour, percentage
/// resistance, floor at zero, stagger if stagger exceeds poise, then knockback when the hit
/// landed unblocked. `evasion_memory` is the target's smoothed evasion state.
pub fn resolve(
    packet: &DamagePacket,
    defence: &Defence,
    evasion_memory: &mut SmoothedState,
    streams: &mut Streams,
) -> DamageResult {
    let missed = DamageResult {
        dealt: Fixed32::ZERO,
        evaded: true,
        blocked: false,
        was_critical: false,
        staggered: false,
        knockback: 0,
    };
    if !packet.tags.contains(Tags::UNAVOIDABLE)
        && streams.roll_smoothed("evasion", defence.evasion, evasion_memory)
    {
        return missed;
    }
    let mut amount = packet.amount.raw() as i64;
    let blocked = defence.blocking;
    if blocked {
        amount = scale(
            amount,
            Fixed32::ONE - defence.block.clamp(Fixed32::ZERO, Fixed32::ONE),
        );
    }
    let was_critical = streams.roll("combat", packet.critical);
    if was_critical {
        amount = scale(amount, packet.critical_multiplier);
    }
    amount -= defence.armour.raw() as i64;
    let resistance = defence.resistances[packet.kind.index()].clamp(Fixed32::ZERO, Fixed32::ONE);
    amount = scale(amount, Fixed32::ONE - resistance);
    let dealt = Fixed32::from_raw(amount.clamp(0, i32::MAX as i64) as i32);
    let staggered = packet.stagger > defence.poise;
    DamageResult {
        dealt,
        evaded: false,
        blocked,
        was_critical,
        staggered,
        knockback: if blocked { 0 } else { packet.knockback },
    }
}

/// `amount × factor` in raw 16.16 units, wide enough not to wrap.
fn scale(amount: i64, factor: Fixed32) -> i64 {
    ((amount as i128 * factor.raw() as i128) >> 16) as i64
}
