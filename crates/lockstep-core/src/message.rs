use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Anything that crosses the boundary or lands in a save.
///
/// Milestone M1 implements this by hand. A derive macro replaces the hand written
/// implementations in milestone M5.
pub trait Message: Serialize + DeserializeOwned + Clone + PartialEq + Debug {
    /// The version of this message shape. A shipped shape is never edited: a change is a new
    /// version plus a conversion.
    const VERSION: u32;
}
