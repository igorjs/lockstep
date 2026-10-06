// SPDX-License-Identifier: Apache-2.0
use lockstep_combat::{angle_between, direction};
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{Column, Handle};
use lockstep_spatial::{line_of_sight, Cell, GridMap, Occupancy, Topology};
use serde::{Deserialize, Serialize};

/// What an agent sees: a cone ahead, and a small circle all around.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cone {
    /// Half the cone's width, either side of the facing.
    pub half_angle: Turn,
    pub range_metres: Fixed32,
    /// Seen whatever the facing, this close.
    pub around_metres: Fixed32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Senses {
    pub sight: Cone,
    /// The farthest the agent hears anything, however loud.
    pub hearing_range_metres: Fixed32,
    /// Eye height in elevation steps, for line of sight.
    pub eye_height: u8,
}

/// An agent seeing a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    pub agent: Handle,
    pub target: Handle,
    pub at: Cell,
}

/// The distance between two cells' centres in metres.
pub fn distance_metres<T: Topology>(
    map: &GridMap<T>,
    from: Cell,
    to: Cell,
    cell_metres: Fixed32,
) -> Fixed32 {
    map.centre(from).distance(map.centre(to)) * cell_metres
}

/// Whether an agent on `from` facing `facing` sees `target`: within the cone's range and angle,
/// or within its circle all around, and with a line of sight at eye height either way.
pub fn sees<T: Topology>(
    map: &GridMap<T>,
    from: Cell,
    facing: Turn,
    senses: &Senses,
    target: Cell,
    cell_metres: Fixed32,
    line: &mut Vec<Cell>,
) -> bool {
    let distance = distance_metres(map, from, target, cell_metres);
    let cone = senses.sight;
    let in_view = distance <= cone.around_metres
        || (distance <= cone.range_metres
            && angle_between(direction(map, from, target), facing) <= cone.half_angle);
    in_view && line_of_sight(map, from, target, senses.eye_height, line)
}

/// Whether an agent checks its senses on this step: agent n checks when `step % every` equals
/// `n % every`, n its slot, so each step checks about one agent in `every`.
pub fn checks_on(agent: Handle, step: u64, every: u32) -> bool {
    let every = every.max(1) as u64;
    step % every == agent.slot_index() as u64 % every
}

/// Every agent whose turn it is this step looks for every target, in handle order of agents then
/// targets. Agents and targets not on the map see and are seen by nothing.
#[allow(clippy::too_many_arguments)]
pub fn perceive<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    senses: &Column<Senses>,
    facing_of: &dyn Fn(Handle) -> Turn,
    targets: &[Handle],
    step: u64,
    every: u32,
    cell_metres: Fixed32,
    out: &mut Vec<Seen>,
) {
    out.clear();
    let mut line = Vec::new();
    let mut targets: Vec<Handle> = targets.to_vec();
    targets.sort();
    for (agent, agent_senses) in senses.iter() {
        if !checks_on(agent, step, every) {
            continue;
        }
        let Some(from) = occupancy.cell_of(agent) else {
            continue;
        };
        let facing = facing_of(agent);
        for target in &targets {
            if *target == agent {
                continue;
            }
            let Some(at) = occupancy.cell_of(*target) else {
                continue;
            };
            if sees(map, from, facing, agent_senses, at, cell_metres, &mut line) {
                out.push(Seen {
                    agent,
                    target: *target,
                    at,
                });
            }
        }
    }
}
