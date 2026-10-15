// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{Handle, Message};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum ControlEvent {
    /// Control passed from a lost entity to the first of its fall-backs.
    Transferred { from: Handle, to: Handle },
    /// The entity in control was lost and no fall-back was left.
    Vacant { from: Handle },
}

/// Who is in control now. Not an entity component: one value for the whole simulation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Control {
    current: Option<Handle>,
}

impl Control {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> Option<Handle> {
        self.current
    }

    /// Puts an entity in control directly.
    pub fn assume(&mut self, who: Handle) {
        self.current = Some(who);
    }

    /// Reports that `lost` has left the simulation. If it was in control, control falls back to
    /// the first of `fall_backs` that is not `lost`, in the order given, and the transfer is
    /// reported; with none, control is vacant. A loss of anyone else changes nothing.
    pub fn lose(&mut self, lost: Handle, fall_backs: &[Handle], events: &mut Vec<ControlEvent>) {
        if self.current != Some(lost) {
            return;
        }
        match fall_backs.iter().find(|candidate| **candidate != lost) {
            Some(&to) => {
                self.current = Some(to);
                events.push(ControlEvent::Transferred { from: lost, to });
            }
            None => {
                self.current = None;
                events.push(ControlEvent::Vacant { from: lost });
            }
        }
    }
}
