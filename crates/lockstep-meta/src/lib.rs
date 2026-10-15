// SPDX-License-Identifier: Apache-2.0
#![doc = include_str!("../README.md")]

mod chronicle;
mod control;
mod envelope;
mod templates;

pub use chronicle::{Chronicle, LineRecord};
pub use control::{Control, ControlEvent};
pub use envelope::{open, seal, EnvelopeError, ENVELOPE_VERSION, MAGIC};
pub use templates::{Archetype, AttributeRule, TemplateError, Templates};
