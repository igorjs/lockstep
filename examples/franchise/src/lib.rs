// SPDX-License-Identifier: Apache-2.0
//! Franchise: a consumer scenario for `lockstep-meta` that is not a game.
//!
//! A franchise keeps a crew of shift staff, grouped into lines that share a family name. One
//! person is the shift manager (in control). Staff are hired from archetypes with a point-buy
//! budget (`data/templates.json`). When the manager departs, control falls back to the newest
//! member of the same line, then to anyone else, and is vacant when no one is left. When the last
//! member of a line departs the line is lost and its name is burned: no later hire may carry it,
//! in this season or any later one. A season is one simulation run; the `Chronicle` outlives it,
//! is saved in a versioned envelope between seasons, and seeds the next season's burned names.

use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_meta::{open, seal, Chronicle, Control, ControlEvent, Templates};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TEMPLATES_JSON: &str = include_str!("../data/templates.json");

/// Family names a hire may carry.
pub const NAMES: [&str; 6] = ["Okafor", "Silva", "Tanaka", "Haddad", "Novak", "Lindqvist"];

pub fn templates() -> Templates {
    Templates::from_json(TEMPLATES_JSON).expect("the committed templates are valid")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    /// The season, counting from 1.
    pub season: u32,
    /// Founding lines and the archetype of each founder. The first founder starts in control.
    pub founders: Vec<(String, String)>,
    /// Names burned in earlier seasons, read from the chronicle.
    pub burned: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Hire {
        line: String,
        archetype: String,
        purchases: Vec<(String, u32)>,
    },
    Depart {
        who: Handle,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    /// The name was burned.
    Burned,
    /// The archetype or the purchases were not allowed.
    Template(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Hired {
        who: Handle,
        line: String,
        generation: u32,
    },
    Refused {
        line: String,
        reason: Refusal,
    },
    Departed {
        who: Handle,
        line: String,
    },
    /// The last member of a line departed; its name is burned. Counts everyone it ever had.
    LineLost {
        line: String,
        members: u32,
    },
    Control(ControlEvent),
    /// Not a member of the crew.
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub line: String,
    pub archetype: String,
    pub attributes: Vec<(String, i32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub season: u32,
    pub crew: StableVector<Member>,
    /// How many members of their line each member was, counting from 1.
    pub generation: Column<u32>,
    pub control: Control,
    /// Everyone each line ever had, by name, kept after the line is lost.
    pub ever: BTreeMap<String, u32>,
    /// Burned names: from earlier seasons, and lost in this one.
    pub burned: Vec<String>,
}

pub struct Franchise {
    world: World,
    templates: Templates,
}

impl Franchise {
    fn with_world(world: World) -> Self {
        Franchise {
            world,
            templates: templates(),
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// The crew in handle order.
    pub fn crew(&self) -> Vec<Handle> {
        self.world.crew.handles()
    }

    pub fn manager(&self) -> Option<Handle> {
        self.world.control.current()
    }

    pub fn is_burned(&self, line: &str) -> bool {
        self.world.burned.iter().any(|name| name == line)
    }

    fn line_of(&self, who: Handle) -> Option<&str> {
        self.world.crew.get(who).map(|member| member.line.as_str())
    }

    fn hire(
        &mut self,
        line: &str,
        archetype: &str,
        purchases: &[(String, u32)],
    ) -> Result<(Handle, u32), Refusal> {
        if self.is_burned(line) {
            return Err(Refusal::Burned);
        }
        let purchases: Vec<(&str, u32)> = purchases
            .iter()
            .map(|(name, points)| (name.as_str(), *points))
            .collect();
        let attributes = self
            .templates
            .build(archetype, &purchases)
            .map_err(|error| Refusal::Template(format!("{error:?}")))?;
        let who = self.world.crew.insert(Member {
            line: line.to_string(),
            archetype: archetype.to_string(),
            attributes,
        });
        let count = self.world.ever.entry(line.to_string()).or_insert(0);
        *count += 1;
        self.world.generation.set(who, *count);
        if self.world.control.current().is_none() {
            self.world.control.assume(who);
        }
        Ok((who, *count))
    }

    /// Who control falls back to: the departed's line, newest first, then everyone else.
    fn fall_backs(&self, line: &str) -> Vec<Handle> {
        let mut same: Vec<Handle> = Vec::new();
        let mut others: Vec<Handle> = Vec::new();
        for (who, member) in self.world.crew.iter() {
            if member.line == line {
                same.push(who);
            } else {
                others.push(who);
            }
        }
        same.sort_by_key(|who| std::cmp::Reverse(self.world.generation.get(*who).copied()));
        same.extend(others);
        same
    }
}

impl Simulation for Franchise {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut franchise = Franchise::with_world(World {
            season: configuration.season,
            crew: StableVector::new(),
            generation: Column::new(),
            control: Control::new(),
            ever: BTreeMap::new(),
            burned: configuration.burned,
        });
        for (line, archetype) in configuration.founders {
            // A founder whose name is burned is not hired, and founders are not bought up.
            let _ = franchise.hire(&line, &archetype, &[]);
        }
        franchise
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        for intent in intents {
            match intent {
                Intent::Hire {
                    line,
                    archetype,
                    purchases,
                } => match self.hire(line, archetype, purchases) {
                    Ok((who, generation)) => context.events.push(Event::Hired {
                        who,
                        line: line.clone(),
                        generation,
                    }),
                    Err(reason) => context.events.push(Event::Refused {
                        line: line.clone(),
                        reason,
                    }),
                },
                Intent::Depart { who } => {
                    let Some(line) = self.line_of(*who).map(str::to_string) else {
                        context.events.push(Event::Unknown);
                        continue;
                    };
                    self.world.crew.remove(*who);
                    self.world.generation.unset(*who);
                    context.events.push(Event::Departed {
                        who: *who,
                        line: line.clone(),
                    });
                    let mut control_events = Vec::new();
                    let fall_backs = self.fall_backs(&line);
                    self.world
                        .control
                        .lose(*who, &fall_backs, &mut control_events);
                    context
                        .events
                        .extend(control_events.into_iter().map(Event::Control));
                    let remaining = self
                        .world
                        .crew
                        .iter()
                        .any(|(_, member)| member.line == line);
                    if !remaining {
                        let members = self.world.ever.get(&line).copied().unwrap_or(0);
                        self.world.burned.push(line.clone());
                        context.events.push(Event::LineLost { line, members });
                    }
                }
            }
        }
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Franchise::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_016;
/// Steps in one season.
pub const DEFAULT_STEPS: u64 = 2_400;
/// Seasons the fixture plays.
pub const SEASONS: u32 = 3;

pub fn founders() -> Vec<(String, String)> {
    ["Okafor", "Silva", "Tanaka", "Haddad"]
        .iter()
        .zip(["lead", "cook", "cashier", "cook"])
        .map(|(line, archetype)| (line.to_string(), archetype.to_string()))
        .collect()
}

/// The configuration of a season: the founders, less any whose name is burned, and the burned
/// names from the chronicle.
pub fn configuration(season: u32, chronicle: &Chronicle) -> Configuration {
    Configuration {
        season,
        founders: founders(),
        burned: chronicle.retired_names(),
    }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Franchise> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 360,
            starting_day: 0,
        },
    )
}

/// The scripted season the fixture hash covers: now and then a hire of a random name (burned or
/// not) with a random archetype and some points, and, a little less often, a random departure.
pub fn script(franchise: &Franchise, script: &mut Streams) -> Vec<Intent> {
    match script.range("act", 0, 40) {
        0 => {
            let archetype = ["cook", "cashier", "lead"][script.pick("archetype", 3)];
            let attribute = ["stamina", "judgement"][script.pick("attribute", 2)];
            vec![Intent::Hire {
                line: NAMES[script.pick("name", NAMES.len())].to_string(),
                archetype: archetype.to_string(),
                purchases: vec![(attribute.to_string(), script.range("points", 0, 4) as u32)],
            }]
        }
        1 | 2 => {
            let crew = franchise.crew();
            if crew.is_empty() {
                Vec::new()
            } else {
                vec![Intent::Depart {
                    who: crew[script.pick("who", crew.len())],
                }]
            }
        }
        _ => Vec::new(),
    }
}

/// Folds a season's events into the chronicle: lost lines are retired, and departures counted.
pub fn record(chronicle: &mut Chronicle, season: u32, events: &[Event]) {
    for event in events {
        match event {
            Event::LineLost { line, members } => {
                chronicle.retire(line, season, *members);
            }
            Event::Departed { .. } => chronicle.add("departures", 1),
            _ => {}
        }
    }
}

/// Plays one season from the chronicle, folds it back, and returns the runner.
pub fn play_season(chronicle: &mut Chronicle, seed: u64, steps: u64) -> Runner<Franchise> {
    let season = chronicle.begin_run();
    let mut runner = runner(configuration(season, chronicle), seed);
    let mut streams = Streams::new(seed ^ 0x0066_7263_6873);
    for _ in 0..steps {
        let intents = script(runner.simulation(), &mut streams);
        let events = runner.step_once(&intents).events;
        record(chronicle, season, &events);
    }
    runner
}

/// Plays `SEASONS` seasons, saving the chronicle in its envelope and opening it between them.
/// Returns the chronicle after the last season and the last season's runner.
pub fn run_fixture(seed: u64, steps: u64) -> (Chronicle, Runner<Franchise>) {
    let mut chronicle = Chronicle::new();
    let mut last = None;
    for season in 0..SEASONS {
        last = Some(play_season(&mut chronicle, seed + season as u64, steps));
        chronicle = open(&seal(&chronicle)).expect("the chronicle just sealed opens");
    }
    (chronicle, last.expect("at least one season"))
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    let (chronicle, runner) = run_fixture(seed, steps);
    lockstep_core::hash_of(&(runner.hash(), chronicle))
}
