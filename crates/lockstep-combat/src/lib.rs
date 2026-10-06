// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod damage;
mod fighter;
mod frame;
mod knockback;
mod shape;

pub use damage::{resolve, DamageKind, DamagePacket, DamageResult, Defence, Tags};
pub use fighter::{
    buffer_seconds, step_combat, ActionDefinition, ActionId, CombatEvent, DodgeDefinition, Fighter,
    Hit, InterruptMask, Moveset, Order, Phase, Refusal,
};
pub use knockback::{knock_back, Impact, Knocked};
pub use shape::{angle_between, direction, hits, HitShape};
