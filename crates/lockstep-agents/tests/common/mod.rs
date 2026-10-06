// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_agents::{
    Cone, Director, Leash, Mind, MindEvent, MindRules, Senses, Stimulus, Surroundings,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, StableVector};
use lockstep_spatial::{GridMap, Occupancy, Square8};

/// Cells are half a metre across.
pub fn half_metre() -> Fixed32 {
    Fixed32::HALF
}

pub fn metres(whole: i32) -> Fixed32 {
    Fixed32::from_int(whole)
}

pub fn minutes(whole: i32) -> Fixed32 {
    Fixed32::from_int(whole)
}

/// Facings, counter-clockwise from east.
pub const EAST: u16 = 0;
pub const NORTH: u16 = 16_384;
pub const WEST: u16 = 32_768;

/// A 12 metre cone 45 degrees either side, 4 metres all around, hearing to 30 metres.
pub fn senses() -> Senses {
    Senses {
        sight: Cone {
            half_angle: 8_192,
            range_metres: metres(12),
            around_metres: metres(4),
        },
        hearing_range_metres: metres(30),
        eye_height: 1,
    }
}

/// An open floor with bodies on the given cells, the first ones listed first in handle order.
pub struct Floor {
    pub map: GridMap<Square8>,
    pub occupancy: Occupancy,
    pub senses: Column<Senses>,
    pub bodies: Vec<Handle>,
}

impl Floor {
    pub fn new(width: u32, height: u32, cells: &[(u32, u32)]) -> Self {
        let map: GridMap<Square8> = GridMap::new(width, height);
        let mut occupancy = Occupancy::new(&map);
        let mut store = StableVector::new();
        let mut senses = Column::new();
        let bodies = cells
            .iter()
            .map(|(x, y)| {
                let body = store.insert(());
                occupancy.place(map.index(*x, *y), body).unwrap();
                senses.set(body, self::senses());
                body
            })
            .collect();
        Floor {
            map,
            occupancy,
            senses,
            bodies,
        }
    }

    pub fn cell(&self, x: u32, y: u32) -> lockstep_spatial::Cell {
        self.map.index(x, y)
    }
}

/// Forgotten after 10 minutes, out of sight for 2 turns Alert to Searching, a noise starts at a
/// half.
pub fn rules() -> MindRules {
    MindRules {
        forget_after_minutes: minutes(10),
        lose_sight_after_minutes: minutes(2),
        heard_confidence: Fixed32::HALF,
    }
}

impl Floor {
    /// Gives every body a fresh mind.
    pub fn minds(&self) -> Column<Mind> {
        let mut minds = Column::new();
        for body in &self.bodies {
            minds.set(*body, Mind::default());
        }
        minds
    }

    /// One `think` of `minutes` with these stimuli, under the common rules.
    pub fn think(
        &self,
        minds: &mut Column<Mind>,
        leashes: &Column<Leash>,
        stimuli: &[(Handle, Stimulus)],
        minutes: Fixed32,
        budget: u16,
    ) -> Vec<MindEvent> {
        self.think_with(&rules(), minds, leashes, stimuli, minutes, budget)
    }

    /// One `think` under the given rules.
    pub fn think_with(
        &self,
        rules: &MindRules,
        minds: &mut Column<Mind>,
        leashes: &Column<Leash>,
        stimuli: &[(Handle, Stimulus)],
        minutes: Fixed32,
        budget: u16,
    ) -> Vec<MindEvent> {
        let mut events = Vec::new();
        let surroundings = Surroundings {
            map: &self.map,
            occupancy: &self.occupancy,
            cell_metres: half_metre(),
        };
        lockstep_agents::think(
            minds,
            leashes,
            stimuli,
            minutes,
            rules,
            &Director {
                alert_budget: budget,
            },
            &surroundings,
            &mut events,
        );
        events
    }
}

/// One game day of weather from a seed at a step rate, where a game day lasts 24 real minutes.
/// Returns the weather after each game minute and every event.
pub fn weather_day(
    seed: u64,
    steps_per_second: u32,
) -> (
    Vec<lockstep_agents::Weather>,
    Vec<lockstep_agents::WeatherEvent>,
) {
    use lockstep_agents::{Weather, WeatherRules};
    use lockstep_core::{Clock, ClockConfiguration, Streams};
    let mut clock = Clock::new(
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 0,
            starting_day: 0,
        },
        1.0 / steps_per_second as f32,
    );
    let rules = WeatherRules {
        mean_strength: metres(6),
        fronts_per_day: 3,
    };
    let mut weather = Weather::new(0, metres(4));
    let mut streams = Streams::new(seed);
    let (mut minutes, mut events) = (Vec::new(), Vec::new());
    // One game minute is one real second here: a day is 1,440 seconds of steps.
    for step in 1..=(1_440 * steps_per_second as u64) {
        let (_, elapsed) = clock.advance_exactly(&mut Vec::new());
        weather.advance(elapsed, &rules, &mut streams, &mut events);
        if step % steps_per_second as u64 == 0 {
            minutes.push(weather.clone());
        }
    }
    (minutes, events)
}
