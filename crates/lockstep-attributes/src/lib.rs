// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod attribute;
mod curve;
mod registry;

pub use attribute::{Attribute, AttributeEvent, Attributes, Modifier, ModifierHandle};
pub use curve::Curve;
pub use registry::{
    AttributeId, Definition, Derived, MaximumPolicy, Registry, RegistryError, Threshold,
};
