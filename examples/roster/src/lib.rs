// SPDX-License-Identifier: Apache-2.0
//! The roster: a consumer scenario for `lockstep-meta` that is not a game.
//!
//! A crew is staffed for one project at a time. Each hire starts from a role and buys extra
//! points of craft and reliability within a budget. One member leads. When the lead leaves, the
//! lead falls back to the next in line, in hire order. A name that has been retired is never
//! hired again, in this project or in any later one: the cross-run record, the chronicle, carries
//! retired names from project to project.

use lockstep_core::{
    hash_of, record_session, ClockConfiguration, Column, Context, Handle, Message, Recorder,
    Recording, RecordingSet, Runner, Session, Simulation, StableVector, StepConfiguration, Streams,
};
use lockstep_meta::{open, seal, Chronicle, Control, ControlEvent, TemplateError, Templates};
use serde::{Deserialize, Serialize};

pub const TEMPLATES_JSON: &str = include_str!("../data/templates.json");

/// The names a project may hire from.
pub const NAME_POOL: [&str; 16] = [
    "Ada", "Bram", "Cleo", "Dara", "Eben", "Faye", "Gus", "Hana", "Ivor", "Jude", "Kira", "Leif",
    "Mina", "Nils", "Orla", "Pavel",
];

pub fn templates() -> Templates {
    Templates::from_json(TEMPLATES_JSON).expect("the committed templates are valid")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    /// The names retired in earlier projects, copied from the chronicle by the host.
    pub retired: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Planner,
    Builder,
}

impl Role {
    fn archetype(self) -> &'static str {
        match self {
            Role::Planner => "planner",
            Role::Builder => "builder",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// Hire a named member into a role, buying points above the role's base.
    Hire {
        name: String,
        role: Role,
        craft_points: u32,
        reliability_points: u32,
    },
    /// A member leaves the project and the name is retired.
    Leave { member: Handle },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    NameRetired,
    NameInUse,
    OverBudget,
    AboveMaximum,
    /// The templates themselves failed, for a reason other than the two above.
    TemplateFailed,
    UnknownMember,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Hired { member: Handle, name: String },
    Left { member: Handle, name: String },
    Refused { reason: Reason },
    Control(ControlEvent),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub names: StableVector<String>,
    /// Craft and reliability, in that order.
    pub ratings: Column<Vec<i32>>,
    /// Members in hire order: the order the lead falls back in.
    pub line: Vec<Handle>,
    pub retired: Vec<String>,
    pub control: Control,
}

pub struct Roster {
    world: World,
    templates: Templates,
}

impl Roster {
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn lead(&self) -> Option<Handle> {
        self.world.control.current()
    }

    pub fn name_of(&self, member: Handle) -> Option<&str> {
        self.world.names.get(member).map(String::as_str)
    }
}

impl Simulation for Roster {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        Roster {
            world: World {
                names: StableVector::new(),
                ratings: Column::new(),
                line: Vec::new(),
                retired: configuration.retired,
                control: Control::new(),
            },
            templates: templates(),
        }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        for intent in intents {
            match intent {
                Intent::Hire {
                    name,
                    role,
                    craft_points,
                    reliability_points,
                } => self.hire(context, name, *role, [*craft_points, *reliability_points]),
                Intent::Leave { member } => self.leave(context, *member),
            }
        }
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Roster {
            world: snapshot,
            templates: templates(),
        }
    }
}

impl Roster {
    fn hire(&mut self, context: &mut Context<'_, Self>, name: &str, role: Role, points: [u32; 2]) {
        let world = &mut self.world;
        let refusal = if world.retired.iter().any(|retired| retired == name) {
            Some(Reason::NameRetired)
        } else if world.names.iter().any(|(_, held)| held == name) {
            Some(Reason::NameInUse)
        } else {
            None
        };
        if let Some(reason) = refusal {
            context.events.push(Event::Refused { reason });
            return;
        }
        let built = self.templates.build(
            role.archetype(),
            &[("craft", points[0]), ("reliability", points[1])],
        );
        let ratings = match built {
            Ok(built) => built.into_iter().map(|(_, value)| value).collect(),
            Err(error) => {
                context.events.push(Event::Refused {
                    reason: refusal_for(&error),
                });
                return;
            }
        };
        let member = world.names.insert(name.to_string());
        world.ratings.set(member, ratings);
        world.line.push(member);
        if world.control.current().is_none() {
            world.control.assume(member);
        }
        context.events.push(Event::Hired {
            member,
            name: name.to_string(),
        });
    }

    fn leave(&mut self, context: &mut Context<'_, Self>, member: Handle) {
        let world = &mut self.world;
        let Some(name) = world.names.remove(member) else {
            context.events.push(Event::Refused {
                reason: Reason::UnknownMember,
            });
            return;
        };
        world.ratings.unset(member);
        world.line.retain(|held| *held != member);
        world.retired.push(name.clone());
        context.events.push(Event::Left { member, name });
        let mut control_events = Vec::new();
        world.control.lose(member, &world.line, &mut control_events);
        context
            .events
            .extend(control_events.into_iter().map(Event::Control));
    }
}

/// The refusal that reports a template error as itself (decision 0068).
pub fn refusal_for(error: &TemplateError) -> Reason {
    match error {
        TemplateError::AboveMaximum(_) => Reason::AboveMaximum,
        TemplateError::OverBudget { .. } => Reason::OverBudget,
        _ => Reason::TemplateFailed,
    }
}

/// The host folds a finished project into the chronicle: every name that left is retired for good
/// and the departures are counted.
pub fn fold(chronicle: &mut Chronicle, run: u32, events: &[Event]) {
    for event in events {
        if let Event::Left { name, .. } = event {
            chronicle.retire(name, run, 1);
            chronicle.add("departures", 1);
        }
    }
}

/// The configuration of the next project: what the chronicle remembers.
pub fn configuration_from(chronicle: &Chronicle) -> Configuration {
    Configuration {
        retired: chronicle.retired_names(),
    }
}

pub const DEFAULT_SEED: u64 = 20_261_019;
pub const DEFAULT_STEPS: u64 = 120;

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Roster> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 300,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: hires from the name pool, some refused, and
/// members leaving.
pub fn script(roster: &Roster, script: &mut Streams) -> Vec<Intent> {
    match script.range("act", 0, 100) {
        0..=29 => vec![Intent::Hire {
            name: NAME_POOL[script.range("name", 0, NAME_POOL.len() as i32) as usize].to_string(),
            role: if script.range("role", 0, 2) == 0 {
                Role::Planner
            } else {
                Role::Builder
            },
            // One hire in eight stretches past a maximum.
            craft_points: script.range("points", 0, 4) as u32
                + if script.range("stretch", 0, 8) == 0 {
                    4
                } else {
                    0
                },
            reliability_points: script.range("points", 0, 3) as u32,
        }],
        30..=37 => {
            let line = &roster.world().line;
            if line.is_empty() {
                Vec::new()
            } else {
                let at = script.range("leaver", 0, line.len() as i32) as usize;
                vec![Intent::Leave { member: line[at] }]
            }
        }
        _ => Vec::new(),
    }
}

/// Runs one project's script on anything that takes steps and returns every event it produced.
fn play<T>(
    session: &mut T,
    seed: u64,
    steps: u64,
    roster: fn(&T) -> &Roster,
    step: fn(&mut T, &[Intent]) -> Vec<Event>,
) -> Vec<Event> {
    let mut streams = Streams::new(seed ^ 0x0000_726f_7374_6572);
    let mut events = Vec::new();
    for _ in 0..steps {
        let intents = script(roster(session), &mut streams);
        events.extend(step(session, &intents));
    }
    events
}

/// One project: runs the script for the steps and returns the runner and every event.
pub fn run_project(
    configuration: Configuration,
    seed: u64,
    steps: u64,
) -> (Runner<Roster>, Vec<Event>) {
    let mut runner = runner(configuration, seed);
    let events = play(
        &mut runner,
        seed,
        steps,
        |runner| runner.simulation(),
        |runner, intents| runner.step_once(intents).events,
    );
    (runner, events)
}

/// The name a recording of this simulation carries.
pub const SIMULATION_ID: &str = "roster";

/// The labels of the two projects in a recording set, in the order they ran.
pub const PROJECTS: [&str; 2] = ["first", "second"];

/// The two projects, each recorded with a checkpoint every `checkpoint_every` steps and, when
/// `keep_snapshots` is set, a snapshot at each, with the hash each project ended on. The second
/// project's recorded configuration is what the chronicle remembered after the first (decision
/// 0067), so a replay needs no chronicle.
fn record_projects(
    seed: u64,
    steps: u64,
    checkpoint_every: u64,
    keep_snapshots: bool,
) -> (Vec<(String, Recording<Intent>)>, Vec<u64>) {
    let mut chronicle = Chronicle::new();
    let mut sessions = Vec::new();
    let mut hashes = Vec::new();
    for (index, label) in PROJECTS.iter().enumerate() {
        let seed = seed ^ index as u64;
        let run = chronicle.begin_run();
        let configuration = configuration_from(&chronicle);
        let mut ended = 0;
        let mut events = Vec::new();
        let recording = record_session(
            SIMULATION_ID,
            &configuration,
            seed,
            self::runner(configuration.clone(), seed),
            checkpoint_every,
            keep_snapshots,
            |recorder: &mut Recorder<Roster>| {
                events = play(
                    recorder,
                    seed,
                    steps,
                    |recorder| recorder.simulation(),
                    |recorder, intents| recorder.step_once(intents).events,
                );
                ended = recorder.runner().hash();
            },
        );
        fold(&mut chronicle, run, &events);
        // The chronicle goes through a save between projects, as a host would keep it.
        chronicle = open(&seal(&chronicle)).expect("the chronicle opens");
        sessions.push((label.to_string(), recording));
        hashes.push(ended);
    }
    (sessions, hashes)
}

/// The same two projects, recorded as a set.
pub fn record_fixture(
    seed: u64,
    steps: u64,
    checkpoint_every: u64,
    keep_snapshots: bool,
) -> RecordingSet<Intent> {
    let (sessions, _) = record_projects(seed, steps, checkpoint_every, keep_snapshots);
    RecordingSet {
        simulation_id: SIMULATION_ID.to_string(),
        sessions,
    }
}

/// Two successive projects. The first starts from an empty chronicle; its departures are folded
/// in, the chronicle is sealed and opened as a save would be, and the second project starts from
/// it. The hash covers the two projects, which is what a replay of the recording reproduces
/// (decision 0067); the second project's configuration holds what the chronicle remembered.
pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    let (_, hashes) = record_projects(seed, steps, steps.max(1), false);
    hash_of(&hashes)
}
