//! Attributes for lockstep-core: every stat is the same machine. A base maximum, modifiers applied
//! Add then Multiply then Override, a clamped current value, thresholds that fire once per
//! crossing, and derived values computed by curves. Health, sanity, hunger, a rover's battery, a
//! credit limit.

mod curve;
mod registry;

pub use curve::Curve;
pub use registry::{
    AttributeId, Definition, Derived, MaximumPolicy, Registry, RegistryError, Threshold,
};
