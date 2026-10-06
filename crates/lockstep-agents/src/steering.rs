// SPDX-License-Identifier: Apache-2.0
use crate::mind::Leash;
use crate::perception::distance_metres;
use lockstep_combat::sidestep_where;
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
/// next cell when the map still lets it step there and no other body holds it; otherwise the
/// shared `sidestep`. Within a leash every cell inside it is allowed; an agent outside its leash
/// (knocked or placed there) may take any cell no farther from home than its own, so it can find
/// its way back. Moving through occupancy reserves the cell at once, so a later agent in the same
/// step never takes it. An agent at the field's goal or out of its reach stays put without an
/// event.
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
        let leash = leashes.get(who).copied();
        let allowed = |cell: Cell| match leash {
            None => true,
            Some(leash) => {
                leash.allows(map, cell, cell_metres)
                    || distance_metres(map, leash.home, cell, cell_metres)
                        <= distance_metres(map, leash.home, from, cell_metres)
            }
        };
        // The map may have changed since the field was built: a closed door is stepped around.
        let open = map.can_step(from, next)
            && occupancy.at(next).is_none_or(|body| body == who)
            && allowed(next);
        let to = if open {
            Some(next)
        } else {
            sidestep_where(map, occupancy, who, from, next, allowed)
        };
        match to {
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
