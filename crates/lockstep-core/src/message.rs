// SPDX-License-Identifier: Apache-2.0
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Anything that crosses the boundary or lands in a save. Derive it with
/// `#[derive(Message)]` and `#[message(version = N)]`, which also derives `Indexable`.
///
/// ```
/// use lockstep_core::{Handle, Indexable, Message};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
/// #[message(version = 2)]
/// enum Event {
///     Opened { account: Handle },
///     Closed { account: Handle, reason: String },
/// }
///
/// assert_eq!(Event::VERSION, 2);
/// ```
///
/// A message without a version does not compile, because a shipped shape is never edited and every
/// change needs a new number:
///
/// ```compile_fail
/// use lockstep_core::Message;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
/// struct Unversioned {
///     count: u32,
/// }
/// ```
pub trait Message: Serialize + DeserializeOwned + Clone + PartialEq + Debug {
    /// The version of this message shape. A shipped shape is never edited: a change is a new
    /// version plus a conversion.
    const VERSION: u32;
}

/// What the timeline indexes an event by: a kind number (an enum's variant index) and the entity
/// handles it mentions. `#[derive(Message)]` writes it.
pub trait Indexable {
    fn kind(&self) -> u16;
    /// Pushes every handle the value mentions, in field order.
    fn handles(&self, out: &mut Vec<crate::Handle>);
}
