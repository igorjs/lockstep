// SPDX-License-Identifier: Apache-2.0
use crate::mind::Leash;
use lockstep_combat::sidestep;
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, Message};
use lockstep_spatial::{Cell, FlowField, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum SteerEvent {
    Stepped {
        who: Handle,
        from: Cell,
        to: Cell,
    },
    /// Its way and both sidesteps were taken, or outside its leash.
    Waited {
        who: Handle,
    },
}

/// Moves each agent one cell down a flow field, in handle order. Each agent takes the field's
/// next cell, or when another body holds it the shared `sidestep`; a cell outside the agent's
/// leash is never taken. Moving through occupancy reserves the cell at once, so a later agent
/// in the same step never takes it, and two bodies never share a cell. An agent at the field's
/// goal or out of its reach stays put without an event.
pub fn steer<T: Topology>(
    map: &GridMap<T>,
    occupancy: &mut Occupancy,
    field: &FlowField,
    agents: &[Handle],
    leashes: &Column<Leash>,
    cell_metres: Fixed32,
    events: &mut Vec<SteerEvent>,
) {
    let mut agents = agents.to_vec();
    agents.sort();
    agents.dedup();
    for who in agents {
        let Some(from) = occupancy.cell_of(who) else {
            continue;
        };
        let Some(next) = field.step_from(from) else {
            continue;
        };
        let allowed = |cell: Cell| {
            leashes
                .get(who)
                .is_none_or(|leash| leash.allows(map, cell, cell_metres))
        };
        let free = occupancy.at(next).is_none_or(|body| body == who);
        let to = if free {
            Some(next)
        } else {
            sidestep(map, occupancy, who, from, next)
        };
        match to.filter(|cell| allowed(*cell)) {
            Some(to) => {
                occupancy
                    .move_to(who, to)
                    .expect("the cell was checked free");
                events.push(SteerEvent::Stepped { who, from, to });
            }
            None => events.push(SteerEvent::Waited { who }),
        }
    }
}
