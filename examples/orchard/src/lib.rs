// SPDX-License-Identifier: Apache-2.0
//! Orchard: a consumer scenario for `lockstep-regions` and `lockstep-calendar` that is not a game.
//!
//! A blight spreads across six orchard blocks (`data/blocks.json`) before the wind from the
//! weather in `lockstep-agents`. The rules are in `data/blight.json`. The calendar
//! (`data/calendar.json`) sets the pace: the blight is idle below 5 degrees and at full pace from
//! 20, so it runs through summer, slows in spring and autumn, and stalls in winter, when the days
//! are also shortest. The host hands each day's daylight to the clock before every step.

use lockstep_agents::{Weather, WeatherRules};
use lockstep_calendar::{Almanac, Calendar, CalendarEvent};
use lockstep_core::math::{Fixed32, Turn};
use lockstep_core::{
    ClockConfiguration, Context, Message, Runner, Simulation, StepConfiguration, Streams,
};
use lockstep_regions::{Breeze, RegionId, Regions, Spread, SpreadEvent, SpreadRules};
use serde::{Deserialize, Serialize};

pub const BLOCKS_JSON: &str = include_str!("../data/blocks.json");
pub const CALENDAR_JSON: &str = include_str!("../data/calendar.json");
pub const BLIGHT_JSON: &str = include_str!("../data/blight.json");

/// Below this many degrees the blight does not spread; from `FULL_PACE_AT` it spreads at full
/// pace, and between the two the pace rises in a straight line.
pub const IDLE_BELOW: i32 = 5;
pub const FULL_PACE_AT: i32 = 20;

pub fn blocks() -> Regions {
    Regions::from_json(BLOCKS_JSON).expect("the committed blocks are valid")
}

pub fn calendar() -> Calendar {
    Calendar::from_json(CALENDAR_JSON).expect("the committed calendar is valid")
}

pub fn blight_rules() -> SpreadRules {
    serde_json::from_str(BLIGHT_JSON).expect("the committed blight rules are valid")
}

/// The pace of the blight at a time of day, 0 to 100 percent, from the temperature.
pub fn pace_percent(calendar: &Calendar, day: u32, minute: u32) -> u32 {
    let degrees = calendar.temperature(day, minute);
    let span = Fixed32::from_int(FULL_PACE_AT - IDLE_BELOW);
    let above = degrees - Fixed32::from_int(IDLE_BELOW);
    (above.raw() as i64 * 100 / span.raw() as i64).clamp(0, 100) as u32
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    /// The wind at the start.
    pub wind_direction: Turn,
    pub wind_strength: Fixed32,
    /// The game day the clock starts on, which sets the season.
    pub starting_day: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// Blight reaches a block from outside.
    Infect { block: RegionId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Blight(SpreadEvent),
    Season(CalendarEvent),
    /// The block is unknown, or already has or had the blight.
    Ignored {
        block: RegionId,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub weather: Weather,
    pub blight: Spread,
    pub almanac: Almanac,
}

pub struct Orchard {
    world: World,
    blocks: Regions,
    calendar: Calendar,
    rules: SpreadRules,
    weather_rules: WeatherRules,
}

impl Orchard {
    fn with_world(world: World) -> Self {
        Orchard {
            world,
            blocks: blocks(),
            calendar: calendar(),
            rules: blight_rules(),
            weather_rules: WeatherRules {
                mean_strength: Fixed32::from_int(4),
                fronts_per_day: 4,
            },
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn blocks(&self) -> &Regions {
        &self.blocks
    }

    pub fn calendar(&self) -> &Calendar {
        &self.calendar
    }

    pub fn block(&self, name: &str) -> RegionId {
        self.blocks.region_id(name).expect("a known block")
    }

    /// Blocks that have or had the blight.
    pub fn reached(&self) -> usize {
        self.world.blight.active_count() + self.world.blight.spent_count()
    }
}

impl Simulation for Orchard {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let blocks = blocks();
        let calendar = calendar();
        Orchard::with_world(World {
            weather: Weather::new(configuration.wind_direction, configuration.wind_strength),
            blight: Spread::new(&blocks),
            almanac: Almanac::new(&calendar, configuration.starting_day),
        })
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut spread_events = Vec::new();
        for intent in intents {
            let Intent::Infect { block } = *intent;
            if !self
                .world
                .blight
                .start(block, &self.rules, &mut spread_events)
            {
                context.events.push(Event::Ignored { block });
            }
        }
        let mut weather_events = Vec::new();
        self.world.weather.advance(
            context.elapsed_minutes,
            &self.weather_rules,
            context.randomness,
            &mut weather_events,
        );
        let wind = self.world.weather.wind();
        let pace = pace_percent(
            &self.calendar,
            context.clock.day(),
            context.clock.whole_minute() as u32,
        );
        self.world.blight.advance(
            &self.blocks,
            &self.rules,
            Breeze {
                direction: wind.direction,
                strength: wind.strength_metres_per_second,
            },
            pace,
            context.elapsed_minutes,
            context.randomness,
            &mut spread_events,
        );
        context
            .events
            .extend(spread_events.into_iter().map(Event::Blight));
        if let Some(season) = self
            .world
            .almanac
            .observe(&self.calendar, context.clock.day())
        {
            context.events.push(Event::Season(season));
        }
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Orchard::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_016;
/// Twelve game days of 3,600 steps: a whole year.
pub const DEFAULT_STEPS: u64 = 43_200;
pub const STEPS_PER_DAY: u64 = 3_600;

/// A light wind blowing east at the start, on the first day of the year.
pub fn orchard_in_spring() -> Configuration {
    starting_on(0)
}

pub fn starting_on(starting_day: u32) -> Configuration {
    Configuration {
        wind_direction: 0,
        wind_strength: Fixed32::from_int(4),
        starting_day,
    }
}

/// A runner starting at 07:00 on the configuration's starting day.
pub fn runner(configuration: Configuration, seed: u64) -> Runner<Orchard> {
    let starting_day = configuration.starting_day;
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 2.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 420,
            starting_day,
        },
    )
}

/// One step, after the host hands the clock the daylight of its day.
pub fn step(runner: &mut Runner<Orchard>, intents: &[Intent]) -> Vec<Event> {
    let day = runner.clock().day();
    let (sunrise, sunset) = runner.simulation().calendar().daylight(day);
    runner.clock_mut().set_daylight(sunrise, sunset);
    runner.step_once(intents).events
}

/// The scripted session the fixture hash covers: the blight reaches the gate on the first step,
/// then, about every quarter of a game day, a random block from outside.
pub fn script(orchard: &Orchard, step_number: u64, script: &mut Streams) -> Vec<Intent> {
    if step_number == 0 {
        return vec![Intent::Infect {
            block: orchard.block("gate"),
        }];
    }
    if script.range("outside", 0, 900) == 0 {
        let block = RegionId(script.pick("block", orchard.blocks().len()) as u16);
        return vec![Intent::Infect { block }];
    }
    Vec::new()
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Orchard> {
    let mut runner = runner(orchard_in_spring(), seed);
    let mut streams = Streams::new(seed ^ 0x006f_7263_6861);
    for step_number in 0..steps {
        let intents = script(runner.simulation(), step_number, &mut streams);
        step(&mut runner, &intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
