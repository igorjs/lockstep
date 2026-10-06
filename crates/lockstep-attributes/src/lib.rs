// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod attribute;
mod curve;
mod effect;
mod registry;

pub use attribute::{Attribute, AttributeEvent, Attributes, Modifier, ModifierHandle};
pub use curve::Curve;
pub use effect::{Effect, EffectContext, EffectEvent, EffectHandle, EffectTag, Effects, Stacking};
pub use registry::{
    AttributeId, Definition, Derived, MaximumPolicy, Registry, RegistryError, Threshold,
};
