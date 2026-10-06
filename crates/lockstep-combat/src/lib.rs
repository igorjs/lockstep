// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod damage;
mod knockback;
mod shape;

pub use damage::{resolve, DamageKind, DamagePacket, DamageResult, Defence, Tags};
pub use knockback::{knock_back, Impact, Knocked};
pub use shape::{angle_between, direction, hits, HitShape};
