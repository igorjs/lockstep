// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod damage;
mod fighter;
mod frame;
mod knockback;
mod movement;
mod projectile;
mod shape;

pub use damage::{resolve, DamageKind, DamagePacket, DamageResult, Defence, Tags};
pub use fighter::{
    buffer_seconds, step_combat, steps_for, ActionDefinition, ActionId, CombatEvent,
    DodgeDefinition, Fighter, Hit, InterruptMask, Moveset, MovesetId, Order, Phase, Refusal,
};
pub use knockback::{knock_back, Impact, Knocked};
pub use movement::{
    quantise_facing, sidestep, sidestep_where, step_movement, Gait, MoveOrder, MovementEvent,
    MovementRules, MovementWorld, Mover,
};
pub use projectile::{step_projectiles, Launch, Projectile, Stopped};
pub use shape::{angle_between, direction, hits, HitShape};
