// SPDX-License-Identifier: Apache-2.0
//! The cargo bay: a consumer scenario for `lockstep-inventory` that is not a game.
//!
//! A crew works a cargo bay with a rack, a cold store and an airlock locker. Deliveries arrive as
//! items from `data/catalogue.json`; rations spoil by the game minute at their store's
//! temperature, so a power cut in the cold store speeds them up. The crew eats rations (a spoiled
//! one makes them sick), wears suits that raise their oxygen, and a faulty seal binds a suit to
//! its wearer and lowers the oxygen until it is repaired. Crew attributes come from
//! `data/attributes.json`.

use lockstep_attributes::{AttributeEvent, AttributeId, Attributes, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_inventory::{
    AffixId, Catalogue, Container, Inventory, InventoryEvent, KindId, Place, Refusal, SlotId,
    Wearers,
};
use serde::{Deserialize, Serialize};

pub const ATTRIBUTES_JSON: &str = include_str!("../data/attributes.json");
pub const CATALOGUE_JSON: &str = include_str!("../data/catalogue.json");

/// Nourishment from a fresh ration, and lost to a spoiled one.
pub const FRESH_MEAL: i32 = 25;
pub const SPOILED_MEAL: i32 = -10;
/// The temperature for items that are loose or worn, in whole degrees.
pub const CABIN_TEMPERATURE: i32 = 18;

pub fn registry() -> Registry {
    Registry::from_json(ATTRIBUTES_JSON).expect("the committed crew attributes are valid")
}

pub fn catalogue(registry: &Registry) -> Catalogue {
    Catalogue::from_json(CATALOGUE_JSON, registry).expect("the committed catalogue is valid")
}

/// A store: a name, its slots, an optional weight limit and its temperature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Store {
    pub name: String,
    pub slots: u16,
    pub weight_limit: Option<i32>,
    pub temperature: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub crew: Vec<String>,
    pub stores: Vec<Store>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    /// New items arrive straight into a store.
    Deliver {
        kind: KindId,
        count: u16,
        to: Handle,
    },
    Move {
        item: Handle,
        to: Handle,
    },
    /// Eats one unit of a food item.
    Eat {
        who: Handle,
        item: Handle,
    },
    Equip {
        who: Handle,
        item: Handle,
    },
    Unequip {
        who: Handle,
        slot: SlotId,
    },
    /// A seal fails on a suit.
    Fault {
        item: Handle,
    },
    Repair {
        item: Handle,
    },
    /// Throws an item out.
    Discard {
        item: Handle,
    },
    /// A store's temperature changes: a power cut, or power back.
    Power {
        store: Handle,
        degrees: i32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Delivered {
        item: Handle,
        into: Handle,
    },
    Moved {
        item: Handle,
        into: Handle,
    },
    Ate {
        who: Handle,
        spoiled: bool,
    },
    Equipped {
        who: Handle,
        item: Handle,
    },
    Unequipped {
        who: Handle,
        item: Handle,
    },
    Faulted {
        item: Handle,
    },
    Repaired {
        item: Handle,
    },
    Discarded {
        item: Handle,
    },
    Powered {
        store: Handle,
        degrees: i32,
    },
    Refused {
        reason: Refusal,
    },
    /// A crew attribute passed a threshold, such as nourishment falling to `hungry`.
    Crossed {
        who: Handle,
        attribute: String,
        threshold: String,
        upward: bool,
    },
    /// The item is not food, or not known.
    NotFood {
        item: Handle,
    },
    Inventory(InventoryEvent),
}

/// Crew members and stores share one store of handles, so a handle names one or the other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Entity {
    Crew(String),
    Store(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub entities: StableVector<Entity>,
    pub inventory: Inventory,
    pub attributes: Column<Attributes>,
}

pub struct CargoBay {
    world: World,
    registry: Registry,
    catalogue: Catalogue,
    nourishment: AttributeId,
    faulty_seal: AffixId,
}

impl CargoBay {
    fn with_world(world: World) -> Self {
        let registry = registry();
        let catalogue = catalogue(&registry);
        CargoBay {
            world,
            nourishment: registry.id("nourishment").expect("defined"),
            faulty_seal: catalogue.affix_id("faulty_seal").expect("defined"),
            registry,
            catalogue,
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The crew, in handle order.
    pub fn crew(&self) -> Vec<Handle> {
        self.with(|entity| matches!(entity, Entity::Crew(_)))
    }

    /// The stores, in handle order.
    pub fn stores(&self) -> Vec<Handle> {
        self.with(|entity| matches!(entity, Entity::Store(_)))
    }

    fn with(&self, keep: impl Fn(&Entity) -> bool) -> Vec<Handle> {
        self.world
            .entities
            .iter()
            .filter(|(_, entity)| keep(entity))
            .map(|(handle, _)| handle)
            .collect()
    }

    /// A crew member's current and maximum value of an attribute.
    pub fn value(&self, who: Handle, name: &str) -> Option<(Fixed32, Fixed32)> {
        let attribute = self.world.attributes.get(who)?.get(self.registry.id(name)?);
        Some((attribute.current(), attribute.maximum()))
    }

    fn intent(&mut self, intent: &Intent, events: &mut Vec<Event>) {
        let mut attribute_events = Vec::new();
        let mut wearers = Wearers {
            attributes: &mut self.world.attributes,
            registry: &self.registry,
            events: &mut attribute_events,
        };
        let inventory = &mut self.world.inventory;
        let catalogue = &self.catalogue;
        let outcome = match intent {
            Intent::Deliver { kind, count, to } => {
                let known = (kind.0 as usize) < catalogue.kind_count()
                    && *count >= 1
                    && *count <= catalogue.kind(*kind).stack;
                if !known {
                    Err(Refusal::Count)
                } else {
                    let item = inventory.create(catalogue, *kind, *count);
                    match inventory.put(item, *to, catalogue) {
                        Ok(into) => {
                            events.push(Event::Delivered {
                                item: into,
                                into: *to,
                            });
                            Ok(())
                        }
                        Err(reason) => {
                            inventory.destroy(item).expect("just made");
                            Err(reason)
                        }
                    }
                }
            }
            Intent::Move { item, to } => {
                // A loose item, such as gear just taken off, is stowed; a stored one moves.
                let moved = match inventory.place(*item) {
                    Some(Place::Loose) => inventory.put(*item, *to, catalogue),
                    _ => inventory.move_between(*item, *to, catalogue),
                };
                moved.map(|moved| {
                    events.push(Event::Moved {
                        item: moved,
                        into: *to,
                    })
                })
            }
            Intent::Eat { who, item } => {
                let food = inventory
                    .item(*item)
                    .map(|held| (catalogue.kind(held.kind).has_tag("food"), held.spoiled));
                match food {
                    Some((true, spoiled)) if wearers.attributes.has(*who) => {
                        inventory.consume(*item, 1).map(|_| {
                            let change = if spoiled { SPOILED_MEAL } else { FRESH_MEAL };
                            wearers.attributes.get_mut(*who).expect("checked").apply(
                                *who,
                                self.nourishment,
                                Fixed32::from_int(change),
                                &self.registry,
                                wearers.events,
                            );
                            events.push(Event::Ate { who: *who, spoiled });
                        })
                    }
                    Some((true, _)) => Err(Refusal::UnknownWearer),
                    _ => {
                        events.push(Event::NotFood { item: *item });
                        Ok(())
                    }
                }
            }
            Intent::Equip { who, item } => inventory
                .equip(*who, *item, catalogue, &mut wearers)
                .map(|_| {
                    events.push(Event::Equipped {
                        who: *who,
                        item: *item,
                    })
                }),
            Intent::Unequip { who, slot } => inventory
                .unequip(*who, *slot, catalogue, &mut wearers)
                .map(|item| events.push(Event::Unequipped { who: *who, item })),
            Intent::Fault { item } => inventory
                .add_affix(*item, self.faulty_seal, catalogue, &mut wearers)
                .map(|_| events.push(Event::Faulted { item: *item })),
            Intent::Repair { item } => inventory
                .remove_affix(*item, self.faulty_seal, catalogue, &mut wearers)
                .map(|_| events.push(Event::Repaired { item: *item })),
            Intent::Discard { item } => inventory
                .destroy(*item)
                .map(|_| events.push(Event::Discarded { item: *item })),
            Intent::Power { store, degrees } => {
                inventory.set_temperature(*store, *degrees).map(|_| {
                    events.push(Event::Powered {
                        store: *store,
                        degrees: *degrees,
                    })
                })
            }
        };
        if let Err(reason) = outcome {
            events.push(Event::Refused { reason });
        }
        for event in attribute_events {
            if let AttributeEvent::Crossed {
                who,
                attribute,
                threshold,
                upward,
            } = event
            {
                events.push(Event::Crossed {
                    who,
                    attribute: self.registry.name(attribute).to_string(),
                    threshold,
                    upward,
                });
            }
        }
    }
}

impl Simulation for CargoBay {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let mut bay = CargoBay::with_world(World {
            entities: StableVector::new(),
            inventory: Inventory::new(),
            attributes: Column::new(),
        });
        for name in configuration.crew {
            let who = bay.world.entities.insert(Entity::Crew(name));
            bay.world
                .attributes
                .set(who, Attributes::from_registry(&bay.registry));
            bay.world.inventory.add_wearer(who, &bay.catalogue);
        }
        for store in configuration.stores {
            let handle = bay.world.entities.insert(Entity::Store(store.name));
            let container = Container::new(
                store.slots,
                store.weight_limit.map(Fixed32::from_int),
                store.temperature,
            );
            bay.world
                .inventory
                .add_container(handle, container)
                .expect("a new store is empty");
        }
        bay
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let mut events = Vec::new();
        for intent in intents {
            self.intent(intent, &mut events);
        }
        let mut spoiled = Vec::new();
        self.world.inventory.spoil(
            context.elapsed_minutes,
            CABIN_TEMPERATURE,
            &self.catalogue,
            &mut spoiled,
        );
        events.extend(spoiled.into_iter().map(Event::Inventory));
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    fn restore(snapshot: World) -> Self {
        CargoBay::with_world(snapshot)
    }
}

pub const DEFAULT_SEED: u64 = 20_261_008;
pub const DEFAULT_STEPS: u64 = 60_000;

/// Three crew, a rack, a cold store and an airlock locker.
pub fn bay() -> Configuration {
    let store = |name: &str, slots, weight_limit, temperature| Store {
        name: name.to_string(),
        slots,
        weight_limit,
        temperature,
    };
    Configuration {
        crew: vec!["ade".into(), "bea".into(), "cy".into()],
        stores: vec![
            store("rack", 8, Some(60), CABIN_TEMPERATURE),
            store("cold store", 6, None, 3),
            store("airlock locker", 4, Some(40), 30),
        ],
    }
}

/// A runner at a step rate, where a game day lasts 24 real minutes: one game minute a real second.
pub fn runner(configuration: Configuration, seed: u64, steps_per_second: u32) -> Runner<CargoBay> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration {
            step_seconds: 1.0 / steps_per_second as f32,
            ..StepConfiguration::default()
        },
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 0,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: now and then a random order, drawn from a
/// separate stream set so the script depends only on the seed and the state.
pub fn order(bay: &CargoBay, script: &mut Streams) -> Option<Intent> {
    if script.range("order", 0, 60) != 0 {
        return None;
    }
    let (crew, stores) = (bay.crew(), bay.stores());
    let catalogue = bay.catalogue();
    let inventory = &bay.world().inventory;
    let who = crew[script.pick("crew", crew.len())];
    let store = stores[script.pick("store", stores.len())];
    let held = inventory
        .container(store)
        .expect("a store")
        .items()
        .to_vec();
    let kind = |name: &str| catalogue.kind_id(name).expect("defined");
    let first = |tag: &str| {
        inventory
            .find_by_tag(store, tag, catalogue)
            .first()
            .copied()
    };
    let suits: Vec<Handle> = inventory
        .items()
        .filter(|(_, item)| item.kind == kind("suit"))
        .map(|(handle, _)| handle)
        .collect();
    let any_suit =
        |script: &mut Streams| (!suits.is_empty()).then(|| suits[script.pick("suit", suits.len())]);
    let spoiled = held
        .iter()
        .copied()
        .find(|item| inventory.item(*item).is_some_and(|held| held.spoiled));
    let deliver = |kind: KindId, count: u16| Intent::Deliver {
        kind,
        count,
        to: store,
    };
    match script.range("kind", 0, 24) {
        0..=3 => Some(deliver(kind("ration"), script.range("count", 1, 7) as u16)),
        4 => Some(deliver(kind("water"), 2)),
        5 => Some(deliver(kind("suit"), 1)),
        6 => Some(deliver(kind("drill"), 1)),
        7..=9 => (!held.is_empty()).then(|| Intent::Move {
            item: held[script.pick("item", held.len())],
            to: stores[script.pick("to", stores.len())],
        }),
        10..=14 => first("food").map(|item| Intent::Eat { who, item }),
        15 | 16 => spoiled
            .or_else(|| (!held.is_empty()).then(|| held[0]))
            .map(|item| Intent::Discard { item }),
        17 => first("gear").map(|item| Intent::Equip { who, item }),
        18 => Some(Intent::Unequip {
            who,
            slot: SlotId(script.range("slot", 0, 2) as u16),
        }),
        19 => any_suit(script).map(|item| Intent::Fault { item }),
        20 => any_suit(script).map(|item| Intent::Repair { item }),
        _ => Some(Intent::Power {
            store: stores[1],
            degrees: if script.range("power", 0, 2) == 0 {
                3
            } else {
                22
            },
        }),
    }
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<CargoBay> {
    let mut runner = runner(bay(), seed, 30);
    let mut script = Streams::new(seed ^ 0x0063_6172_676f);
    for _ in 0..steps {
        let intents: Vec<Intent> = order(runner.simulation(), &mut script)
            .into_iter()
            .collect();
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}

/// Whether an item is worn by someone.
pub fn is_worn(bay: &CargoBay, item: Handle) -> bool {
    matches!(bay.world().inventory.place(item), Some(Place::Worn { .. }))
}
