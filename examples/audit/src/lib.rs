// SPDX-License-Identifier: Apache-2.0
//! The audit: a consumer scenario for `lockstep-knowledge` that is not a game.
//!
//! Accounts move money. An auditor reviews every transfer as it happens and picks up fragments
//! of evidence: a round amount, a transfer in the small hours, and a receiver that three or more
//! senders pay. The facts, the question they open and the findings they fire are data in
//! `data/knowledge.json`. The first question opens after two fragments, and every finding fires
//! once, across saves too.

use lockstep_core::{
    ClockConfiguration, Column, Context, Handle, Message, Runner, Simulation, StableVector,
    StepConfiguration, Streams,
};
use lockstep_knowledge::{
    enrol, evaluate, notice, receive, Catalogue, Fragment, Knowledge, KnowledgeEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KNOWLEDGE_JSON: &str = include_str!("../data/knowledge.json");

/// The event kinds the catalogue names: each event's place in `Event`.
pub const EVENT_KINDS: &[(&str, u16)] = &[("transferred", 0), ("reviewed", 1)];

/// A round amount is a whole hundred, in minor units.
pub const ROUND_MINOR: i64 = 10_000;
/// Transfers before this game minute of the day are in the small hours.
pub const SMALL_HOURS_END: u16 = 300;
/// A receiver paid by this many distinct senders is a shared receiver.
pub const SHARED_SENDERS: usize = 3;

pub fn catalogue() -> Catalogue {
    Catalogue::from_json(KNOWLEDGE_JSON, EVENT_KINDS).expect("the committed knowledge is valid")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct Configuration {
    pub accounts: Vec<String>,
    pub auditor: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Intent {
    Transfer {
        from: Handle,
        to: Handle,
        amount_minor: i64,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub enum Event {
    Transferred {
        from: Handle,
        to: Handle,
        amount_minor: i64,
        number: u32,
    },
    Reviewed {
        auditor: Handle,
        number: u32,
    },
    Knowledge(KnowledgeEvent),
    /// Not between two different accounts, or not a positive amount.
    Refused,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Party {
    Account(String),
    Auditor(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
pub struct World {
    pub parties: StableVector<Party>,
    pub knowledge: Column<Knowledge>,
    /// Distinct senders seen paying each receiver.
    pub senders: BTreeMap<Handle, BTreeSet<Handle>>,
    pub transfers: u32,
    /// The catalogue this knowledge was gathered with; a load checks it.
    pub catalogue_hash: u64,
}

pub struct Audit {
    world: World,
    catalogue: Catalogue,
}

impl Audit {
    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    pub fn accounts(&self) -> Vec<Handle> {
        self.of(|party| matches!(party, Party::Account(_)))
    }

    pub fn auditor(&self) -> Handle {
        self.of(|party| matches!(party, Party::Auditor(_)))[0]
    }

    fn of(&self, keep: impl Fn(&Party) -> bool) -> Vec<Handle> {
        self.world
            .parties
            .iter()
            .filter(|(_, party)| keep(party))
            .map(|(handle, _)| handle)
            .collect()
    }

    fn is_account(&self, who: Handle) -> bool {
        matches!(self.world.parties.get(who), Some(Party::Account(_)))
    }

    /// What the auditor makes of one transfer: a fragment for each pattern it shows.
    fn review(
        &mut self,
        from: Handle,
        to: Handle,
        amount_minor: i64,
        minute: u16,
    ) -> Vec<Fragment> {
        let number = self.world.transfers;
        let id = |name: &str| self.catalogue.fact_id(name).expect("defined");
        let mut fragments = Vec::new();
        if amount_minor % ROUND_MINOR == 0 {
            fragments.push(Fragment {
                fact: id("round_amounts"),
                source: number,
            });
        }
        if minute < SMALL_HOURS_END {
            fragments.push(Fragment {
                fact: id("night_transfers"),
                source: number,
            });
        }
        let senders = self.world.senders.entry(to).or_default();
        senders.insert(from);
        if senders.len() >= SHARED_SENDERS {
            // One source per receiver: the same receiver never counts twice.
            fragments.push(Fragment {
                fact: id("shared_receiver"),
                source: to.slot_index(),
            });
        }
        fragments
    }
}

impl Simulation for Audit {
    type Intent = Intent;
    type Event = Event;
    type Snapshot = World;
    type Configuration = Configuration;

    fn create(configuration: Configuration, _randomness: &mut Streams) -> Self {
        let catalogue = catalogue();
        let mut world = World {
            parties: StableVector::new(),
            knowledge: Column::new(),
            senders: BTreeMap::new(),
            transfers: 0,
            catalogue_hash: catalogue.hash(),
        };
        for name in configuration.accounts {
            world.parties.insert(Party::Account(name));
        }
        let auditor = world.parties.insert(Party::Auditor(configuration.auditor));
        enrol(&mut world.knowledge, auditor);
        Audit { world, catalogue }
    }

    fn step(&mut self, context: &mut Context<'_, Self>, intents: &[Intent]) {
        let auditor = self.auditor();
        let minute = context.clock.whole_minute();
        let mut events = Vec::new();
        let mut knowledge_events = Vec::new();
        for intent in intents {
            let Intent::Transfer {
                from,
                to,
                amount_minor,
            } = *intent;
            if from == to || amount_minor <= 0 || !self.is_account(from) || !self.is_account(to) {
                events.push(Event::Refused);
                continue;
            }
            self.world.transfers += 1;
            let number = self.world.transfers;
            events.push(Event::Transferred {
                from,
                to,
                amount_minor,
                number,
            });
            for fragment in self.review(from, to, amount_minor, minute) {
                receive(
                    &mut self.world.knowledge,
                    auditor,
                    fragment,
                    &self.catalogue,
                    &mut knowledge_events,
                );
            }
            events.push(Event::Reviewed { auditor, number });
        }
        notice(&mut self.world.knowledge, &events);
        evaluate(
            &mut self.world.knowledge,
            &self.catalogue,
            &mut knowledge_events,
        );
        events.extend(knowledge_events.into_iter().map(Event::Knowledge));
        context.events.extend(events);
    }

    fn snapshot(&self) -> World {
        self.world.clone()
    }

    /// Panics when the save was made with another catalogue: its fact, question and rule ids
    /// would point at different entries.
    fn restore(snapshot: World) -> Self {
        let catalogue = catalogue();
        assert_eq!(
            snapshot.catalogue_hash,
            catalogue.hash(),
            "the save was made with another catalogue"
        );
        Audit {
            world: snapshot,
            catalogue,
        }
    }
}

pub const DEFAULT_SEED: u64 = 20_261_010;
pub const DEFAULT_STEPS: u64 = 60_000;

pub fn books() -> Configuration {
    Configuration {
        accounts: ["ada", "bo", "cai", "dee", "eli", "fay", "gus", "hal"]
            .map(String::from)
            .to_vec(),
        auditor: "ivy".into(),
    }
}

/// A day lasts 24 real minutes, starting at ten in the morning.
pub fn runner(configuration: Configuration, seed: u64) -> Runner<Audit> {
    Runner::new(
        configuration,
        seed,
        StepConfiguration::default(),
        ClockConfiguration {
            day_length_real_minutes: 24.0,
            sunrise_minute: 360,
            sunset_minute: 1_080,
            starting_minute: 600,
            starting_day: 0,
        },
    )
}

/// The scripted session the fixture hash covers: now and then a transfer between two random
/// accounts, one in eight of them a round amount.
pub fn script(audit: &Audit, script: &mut Streams) -> Vec<Intent> {
    if script.range("transfer", 0, 40) != 0 {
        return Vec::new();
    }
    let accounts = audit.accounts();
    let from = accounts[script.pick("from", accounts.len())];
    let to = accounts[script.pick("to", accounts.len())];
    let amount_minor = if script.range("round", 0, 8) == 0 {
        script.range("hundreds", 1, 20) as i64 * ROUND_MINOR
    } else {
        script.range("amount", 1, 99_999) as i64
    };
    vec![Intent::Transfer {
        from,
        to,
        amount_minor,
    }]
}

pub fn run_fixture(seed: u64, steps: u64) -> Runner<Audit> {
    let mut runner = runner(books(), seed);
    let mut streams = Streams::new(seed ^ 0x0061_7564_6974);
    for _ in 0..steps {
        let intents = script(runner.simulation(), &mut streams);
        runner.step_once(&intents);
    }
    runner
}

pub fn fixture_hash(seed: u64, steps: u64) -> u64 {
    run_fixture(seed, steps).hash()
}
