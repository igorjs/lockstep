// SPDX-License-Identifier: Apache-2.0
//! Training: a consumer scenario for `lockstep-progression` that is not a game.
//!
//! Workshop technicians earn points by working shifts and handling incidents, and gain
//! experience as they do. They spend points on a six-node web in `data/training.json`: basics,
//! then electrical or mechanical (each excludes the other), diagnostics, senior (experience at
//! least 50) and chief, a keystone (experience at least 80). Each node raises an attribute from
//! `data/attributes.json`. A technician may unlearn a node for half its cost back, but never the
//! keystone or a node another one needs.

use lockstep_attributes::{AttributeEvent, AttributeId, Attributes, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_progression::{
    award, enrol, refund, unlock, why_not, Graph, NodeId, Owners, Progress, ProgressEvent, Refusal,
    TriggerId,
};
use serde::{Deserialize, Serialize};

pub const ATTRIBUTES_JSON: &str = include_str!("../data/attributes.json");
pub const TRAINING_JSON: &str = include_str!("../data/training.json");

/// Experience a shift and an incident add.
pub const SHIFT_EXPERIENCE: i32 = 2;
pub const INCIDENT_EXPERIENCE: i32 = 5;

pub fn registry() -> Registry {
    Registry::from_json(ATTRIBUTES_JSON).expect("the committed attributes are valid")
}

pub fn graph(registry: &Registry) -> Graph {
    Graph::from_json(TRAINING_JSON, registry).expect("the committed training web is valid")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub technicians: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// A shift or an incident, by trigger.
    Work {
        technician: Handle,
        trigger: TriggerId,
    },
    Study {
        technician: Handle,
        node: NodeId,
    },
    Unlearn {
        technician: Handle,
        node: NodeId,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Progress(ProgressEvent),
    Refused {
        technician: Handle,
        reason: Refusal,
    },
    /// Experience passed a named mark: `seasoned` or `veteran`.
    Mark {
        technician: Handle,
        mark: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub technicians: StableVector<String>,
    pub progress: Column<Progress>,
    pub attributes: Column<Attributes>,
}

pub struct Training {
    world: World,
    registry: Registry,
    graph: Graph,
    experience: AttributeId,
}

impl Training {
    fn with_world(world: World) -> Self {
        let registry = registry();
        let graph = graph(&registry);
        Training {
            world,
            experience: registry.id("experience").expect("defined"),
            registry,
            graph,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn technicians(&self) -> Vec<Handle> {
        self.world.technicians.handles()
    }

    /// A technician's current value of an attribute.
    pub fn value(&self, technician: Handle, name: &str) -> Option<Fixed32> {
        let id = self.registry.id(name)?;
        Some(self.world.attributes.get(technician)?.get(id).current())
    }

    /// The nodes a technician could take now, in node order.
    pub fn available(&self, technician: Handle) -> Vec<NodeId> {
        (0..self.graph.nodes.len() as u16)
            .map(NodeId)
            .filter(|node| {
                why_not(
                    &self.world.progress,
                    technician,
                    *node,
                    &self.graph,
                    &self.world.attributes,
                )
                .is_none()
            })
            .collect()
    }

    fn intent(
        &mut self,
        intent: &Intent,
        events: &mut Vec<Event>,
        attribute_events: &mut Vec<AttributeEvent>,
    ) {
        let mut progress_events = Vec::new();
        let (technician, outcome) = match *intent {
            Intent::Work {
                technician,
                trigger,
            } => {
                let outcome = award(
                    &mut self.world.progress,
                    technician,
                    trigger,
                    &self.graph,
                    &mut progress_events,
                );
                if outcome.is_ok() {
                    let incident = self.graph.trigger_id("incident") == Some(trigger);
                    let gained = if incident {
                        INCIDENT_EXPERIENCE
                    } else {
                        SHIFT_EXPERIENCE
                    };
                    if let Some(attributes) = self.world.attributes.get_mut(technician) {
                        attributes.apply(
                            technician,
                            self.experience,
                            Fixed32::from_int(gained),
                            &self.registry,
                            attribute_events,
                        );
                    }
                }
                (technician, outcome)
            }
            Intent::Study { technician, node } => {
                let mut owners = Owners {
                    attributes: &mut self.world.attributes,
                    registry: &self.registry,
                    events: attribute_events,
                };
                let outcome = unlock(
                    &mut self.world.progress,
                    technician,
                    node,
                    &self.graph,
                    &mut owners,
                    &mut progress_events,
                );
                (technician, outcome)
            }
            Intent::Unlearn { technician, node } => {
                let mut owners = Owners {
                    attributes: &mut self.world.attributes,
                    registry: &self.registry,
                    events: attribute_events,
                };
                let outcome = refund(
                    &mut self.world.progress,
                    technician,
                    node,
                    &self.graph,
                    &mut owners,
                    &mut progress_events,
                )
                .map(|_| ());
                (technician, outcome)
            }
        };
        if let Err(reason) = outcome {
            events.push(Event::Refused { technician, reason });
        }
        events.extend(progress_events.into_iter().map(Event::Progress));
    }
}

impl Simulation for Training {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut training = Training::with_world(World {
            technicians: StableVector::new(),
            progress: Column::new(),
            attributes: Column::new(),
        });
        for name in configuration.technicians {
            let technician = training.world.technicians.insert(name);
            enrol(&mut training.world.progress, technician);
            training
                .world
                .attributes
                .set(technician, Attributes::from_registry(&training.registry));
        }
        training
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        let mut attribute_events = Vec::new();
        for intent in intents {
            self.intent(intent, &mut events, &mut attribute_events);
        }
        for event in attribute_events {
            if let AttributeEvent::Crossed {
                who,
                attribute,
                threshold,
                upward: true,
            } = event
            {
                if attribute == self.experience {
                    events.push(Event::Mark {
                        technician: who,
                        mark: threshold,
                    });
                }
            }
        }
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Training::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_011;
pub const DEFAULT_STEPS: u64 = 30_000;

pub fn workshop() -> Configuration {
    Configuration {
        technicians: ["ana", "ben", "cal", "dot"].map(String::from).to_vec(),
    }
}

pub fn runner(configuration: Configuration, seed: u64) -> Runner<Training> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 480,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers. Now and then a random technician works a
/// shift (or, one time in five, an incident), studies a random node it can take, or, rarely,
/// unlearns a random node it holds.
pub fn script(training: &Training, script: &mut Streams) -> Vec<Intent> {
    if script.range("act", 0, 30) != 0 {
        return Vec::new();
    }
    let technicians = training.technicians();
    let technician = technicians[script.pick("technician", technicians.len())];
    let graph = training.graph();
    match script.range("kind", 0, 20) {
        0..=11 => {
            let name = if script.range("incident", 0, 5) == 0 {
                "incident"
            } else {
                "shift"
            };
            vec![Intent::Work {
                technician,
                trigger: graph.trigger_id(name).expect("defined"),
            }]
        }
        12..=18 => {
            let available = training.available(technician);
            if available.is_empty() {
                return Vec::new();
            }
            vec![Intent::Study {
                technician,
                node: available[script.pick("node", available.len())],
            }]
        }
        _ => {
            let held: Vec<NodeId> = training
                .world()
                .progress
                .get(technician)
                .map(|progress| progress.taken().collect())
                .unwrap_or_default();
            if held.is_empty() {
                return Vec::new();
            }
            vec![Intent::Unlearn {
                technician,
                node: held[script.pick("node", held.len())],
            }]
        }
    }
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Training> {
    let mut runner = runner(workshop(), seed);
    let mut streams = Streams::new(seed ^ 0x0074_7261_696e);
    for _ in 0..steps {
        let intents = script(runner.simulation(), &mut streams);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
