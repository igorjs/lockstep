// SPDX-License-Identifier: Apache-2.0
//! The drone fleet: a consumer scenario that is not a game.
//!
//! A fleet of delivery drones flies from one depot. Each drone's battery is three attributes read
//! from `data/attributes.json`: `wear` (a ratchet, so each wear mark is a floor that service cannot
//! undo), `capacity` (derived from wear), and `charge` (whose maximum follows capacity). A flight
//! is an effect that drains charge for its length, docking is an effect that charges until the
//! next launch, and every launch risks a fault on a `Chance` roll that adds wear. A drone that runs
//! dry in the air is stranded until it is recovered.

use lockstep_attributes::{
    AttributeEvent, AttributeId, Attributes, Effect, EffectContext, EffectEvent, EffectTag,
    Effects, Modifier, ModifierHandle, Registry, Stacking,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{
    Chance, ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use serde::{Deserialize, Serialize};

/// The battery, as data.
pub const ATTRIBUTES_JSON: &str = include_str!("../data/attributes.json");

/// The chance that a launch causes a fault.
pub const FAULT_CHANCE: Chance = Chance(500);
/// Wear from a fault, and from a recovery after running dry.
pub const FAULT_WEAR: i32 = 12;
pub const RECOVERY_WEAR: i32 = 8;
/// Wear a service removes, down to the last wear mark passed.
pub const SERVICE_REPAIR: i32 = 20;
/// Charge used per minute of flight, and gained per minute docked.
pub const FLIGHT_DRAIN: i32 = 2;
pub const DOCK_CHARGE: i32 = 4;
/// The lowest charge a drone may launch with.
pub const LAUNCH_FLOOR: i32 = 25;
/// The wear at which a drone is retired: it never launches again.
pub const RETIRE_WEAR: i32 = 75;

pub fn registry() -> Registry {
    Registry::from_json(ATTRIBUTES_JSON).expect("the committed battery data is valid")
}

#[derive(Clone, Copy, Debug)]
pub struct Ids {
    pub wear: AttributeId,
    pub capacity: AttributeId,
    pub charge: AttributeId,
}

impl Ids {
    pub fn of(registry: &Registry) -> Self {
        let id = |name: &str| {
            registry
                .id(name)
                .expect("the battery attributes are defined")
        };
        Ids {
            wear: id("wear"),
            capacity: id("capacity"),
            charge: id("charge"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Docked,
    Flying,
    /// Ran dry in the air; waits for a recovery.
    Stranded,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub drones: u32,
}

impl Message for Configuration {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    /// Fly for this many game minutes.
    Launch { drone: Handle, minutes: u16 },
    /// Come home early, or bring a stranded drone back (which adds wear).
    Dock { drone: Handle },
    /// Repair wear while docked, but never below the last wear mark passed.
    Service { drone: Handle },
}

impl Message for Intent {
    const VERSION: u32 = 1;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Refusal {
    NotDocked,
    TooLow,
    /// Wear reached the `retire` mark.
    Retired,
    NotFlying,
    UnknownDrone,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Launched {
        drone: Handle,
    },
    Fault {
        drone: Handle,
    },
    Landed {
        drone: Handle,
    },
    Docked {
        drone: Handle,
    },
    Stranded {
        drone: Handle,
    },
    Serviced {
        drone: Handle,
    },
    /// The charge fell below a named level: `low` or `critical`.
    Battery {
        drone: Handle,
        level: String,
    },
    /// The wear passed a named mark: `worn`, `tired` or `retire`.
    Wear {
        drone: Handle,
        mark: String,
    },
    Refused {
        drone: Handle,
        reason: Refusal,
    },
}

impl Message for Event {
    const VERSION: u32 = 1;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub drones: StableVector<String>,
    pub status: Column<Status>,
    pub batteries: Column<Attributes>,
    pub effects: Effects,
    /// The modifier that holds each charge maximum at its capacity.
    pub capacity_limits: Column<ModifierHandle>,
}

impl Message for World {
    const VERSION: u32 = 1;
}

pub struct Fleet {
    world: World,
    registry: Registry,
    ids: Ids,
}

fn flight(ids: Ids, minutes: u16) -> Effect {
    Effect {
        attribute: ids.charge,
        modifier: None,
        per_minute: Some(Fixed32::from_int(-FLIGHT_DRAIN)),
        remaining_minutes: Some(Fixed32::from_int(minutes as i32)),
        tag: EffectTag::new("flight"),
        stacking: Stacking::Replace,
    }
}

fn charging(ids: Ids) -> Effect {
    Effect {
        attribute: ids.charge,
        modifier: None,
        per_minute: Some(Fixed32::from_int(DOCK_CHARGE)),
        remaining_minutes: None,
        tag: EffectTag::new("charging"),
        stacking: Stacking::Replace,
    }
}

impl Fleet {
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn ids(&self) -> Ids {
        self.ids
    }

    pub fn value(&self, drone: Handle, attribute: AttributeId) -> Option<Fixed32> {
        Some(self.world.batteries.get(drone)?.get(attribute).current())
    }

    pub fn maximum(&self, drone: Handle, attribute: AttributeId) -> Option<Fixed32> {
        Some(self.world.batteries.get(drone)?.get(attribute).maximum())
    }

    fn with_world(world: World) -> Self {
        let registry = registry();
        let ids = Ids::of(&registry);
        Fleet {
            world,
            registry,
            ids,
        }
    }

    /// Holds the charge maximum at the capacity, replacing the old limit.
    fn limit_charge(&mut self, drone: Handle, events: &mut Vec<AttributeEvent>) {
        let Some(battery) = self.world.batteries.get_mut(drone) else {
            return;
        };
        let capacity = battery.get(self.ids.capacity).current();
        if let Some(old) = self.world.capacity_limits.unset(drone) {
            battery.remove_modifier(drone, self.ids.charge, old, &self.registry, events);
        }
        let limit = battery.add_modifier(
            drone,
            self.ids.charge,
            Modifier::Override(capacity),
            &self.registry,
            events,
        );
        self.world.capacity_limits.set(drone, limit);
    }

    fn change_wear(&mut self, drone: Handle, amount: i32, events: &mut Vec<AttributeEvent>) {
        if let Some(battery) = self.world.batteries.get_mut(drone) {
            battery.apply(
                drone,
                self.ids.wear,
                Fixed32::from_int(amount),
                &self.registry,
                events,
            );
        }
        self.limit_charge(drone, events);
    }

    fn intent(
        &mut self,
        intent: &Intent,
        context: &mut Context<'_, Self>,
        events: &mut Vec<AttributeEvent>,
    ) {
        let drone = match intent {
            Intent::Launch { drone, .. } | Intent::Dock { drone } | Intent::Service { drone } => {
                *drone
            }
        };
        let Some(status) = self.world.status.get(drone).copied() else {
            context.events.push(Event::Refused {
                drone,
                reason: Refusal::UnknownDrone,
            });
            return;
        };
        let refusal = match intent {
            Intent::Launch { .. } if status != Status::Docked => Some(Refusal::NotDocked),
            Intent::Launch { .. }
                if self.value(drone, self.ids.wear) >= Some(Fixed32::from_int(RETIRE_WEAR)) =>
            {
                Some(Refusal::Retired)
            }
            Intent::Launch { .. }
                if self.value(drone, self.ids.charge) < Some(Fixed32::from_int(LAUNCH_FLOOR)) =>
            {
                Some(Refusal::TooLow)
            }
            Intent::Dock { .. } if status == Status::Docked => Some(Refusal::NotFlying),
            Intent::Service { .. } if status != Status::Docked => Some(Refusal::NotDocked),
            _ => None,
        };
        if let Some(reason) = refusal {
            context.events.push(Event::Refused { drone, reason });
            return;
        }
        match intent {
            Intent::Launch { minutes, .. } => {
                let mut effect_events = Vec::new();
                let mut effect_context = EffectContext {
                    attributes: &mut self.world.batteries,
                    registry: &self.registry,
                    attribute_events: &mut *events,
                    effect_events: &mut effect_events,
                };
                let effects = &mut self.world.effects;
                effects.remove_by_tag(drone, &EffectTag::new("charging"), &mut effect_context);
                effects.add(drone, flight(self.ids, *minutes), &mut effect_context);
                self.world.status.set(drone, Status::Flying);
                context.events.push(Event::Launched { drone });
                if context.randomness.roll("faults", FAULT_CHANCE) {
                    context.events.push(Event::Fault { drone });
                    self.change_wear(drone, FAULT_WEAR, events);
                }
            }
            Intent::Dock { .. } => {
                if status == Status::Stranded {
                    self.change_wear(drone, RECOVERY_WEAR, events);
                }
                self.dock(drone, events);
                context.events.push(Event::Docked { drone });
            }
            Intent::Service { .. } => {
                self.change_wear(drone, -SERVICE_REPAIR, events);
                context.events.push(Event::Serviced { drone });
            }
        }
    }

    fn dock(&mut self, drone: Handle, events: &mut Vec<AttributeEvent>) {
        let mut effect_events = Vec::new();
        let mut effect_context = EffectContext {
            attributes: &mut self.world.batteries,
            registry: &self.registry,
            attribute_events: events,
            effect_events: &mut effect_events,
        };
        let effects = &mut self.world.effects;
        effects.remove_by_tag(drone, &EffectTag::new("flight"), &mut effect_context);
        effects.add(drone, charging(self.ids), &mut effect_context);
        self.world.status.set(drone, Status::Docked);
    }

    /// Runs every effect for the step's minutes, then reads what happened: a flight that ran
    /// its course lands and docks, a drone that runs dry in the air is stranded, and charge levels
    /// and wear marks become events.
    fn tick(&mut self, context: &mut Context<'_, Self>, events: &mut Vec<AttributeEvent>) {
        let mut effect_events = Vec::new();
        let mut effect_context = EffectContext {
            attributes: &mut self.world.batteries,
            registry: &self.registry,
            attribute_events: &mut *events,
            effect_events: &mut effect_events,
        };
        self.world.effects.tick(
            context.elapsed_minutes,
            context.clock.day(),
            &mut effect_context,
        );
        for event in effect_events {
            if let EffectEvent::Expired { who, tag } = event {
                if tag == EffectTag::new("flight") {
                    context.events.push(Event::Landed { drone: who });
                    self.dock(who, events);
                }
            }
        }
        let mut stranded = Vec::new();
        for event in std::mem::take(events) {
            match event {
                AttributeEvent::Crossed {
                    who,
                    attribute,
                    threshold,
                    upward: false,
                } if attribute == self.ids.charge => context.events.push(Event::Battery {
                    drone: who,
                    level: threshold,
                }),
                AttributeEvent::Crossed {
                    who,
                    attribute,
                    threshold,
                    upward: true,
                } if attribute == self.ids.wear => context.events.push(Event::Wear {
                    drone: who,
                    mark: threshold,
                }),
                AttributeEvent::Emptied { who, attribute }
                    if attribute == self.ids.charge
                        && self.world.status.get(who) == Some(&Status::Flying) =>
                {
                    stranded.push(who);
                }
                _ => {}
            }
        }
        for drone in stranded {
            let mut effect_events = Vec::new();
            let mut effect_context = EffectContext {
                attributes: &mut self.world.batteries,
                registry: &self.registry,
                attribute_events: &mut Vec::new(),
                effect_events: &mut effect_events,
            };
            self.world
                .effects
                .remove_by_tag(drone, &EffectTag::new("flight"), &mut effect_context);
            self.world.status.set(drone, Status::Stranded);
            context.events.push(Event::Stranded { drone });
        }
    }
}

impl Simulation for Fleet {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut fleet = Fleet::with_world(World {
            drones: StableVector::new(),
            status: Column::new(),
            batteries: Column::new(),
            effects: Effects::new(),
            capacity_limits: Column::new(),
        });
        for index in 0..configuration.drones {
            let drone = fleet.world.drones.insert(format!("drone-{index}"));
            fleet
                .world
                .batteries
                .set(drone, Attributes::from_registry(&fleet.registry));
            let mut events = Vec::new();
            fleet.limit_charge(drone, &mut events);
            fleet.dock(drone, &mut events);
        }
        fleet
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        for intent in intents {
            self.intent(intent, context, &mut events);
        }
        self.tick(context, &mut events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        Fleet::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_006;
pub const DEFAULT_STEPS: u64 = 60_000;
pub const DRONES: u32 = 6;

pub fn runner(seed: u64) -> Runner<Fleet> {
    Runner::new(
        Configuration { drones: DRONES },
        seed,
        StepConfiguration::default(),
        // One game minute per real second: a step is a thirtieth of a minute.
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 0,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: now and then a random order for a random drone,
/// drawn from a separate stream set so the script depends only on the seed.
pub fn run_fixture(seed: u64, steps: u64) -> Runner<Fleet> {
    let mut runner = runner(seed);
    let drones = runner.simulation().world().drones.handles();
    let mut script = Streams::new(seed ^ 0x6472_6f6e_6573);
    for _ in 0..steps {
        let mut intents = Vec::new();
        if script.range("order", 0, 40) == 0 {
            let drone = drones[script.pick("drone", drones.len())];
            intents.push(match script.range("kind", 0, 10) {
                0..=5 => Intent::Launch {
                    drone,
                    minutes: script.range("minutes", 10, 70) as u16,
                },
                6..=8 => Intent::Dock { drone },
                _ => Intent::Service { drone },
            });
        }
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
