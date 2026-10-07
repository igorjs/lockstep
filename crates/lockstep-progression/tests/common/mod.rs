// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_attributes::{AttributeEvent, Attributes, Registry};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, StableVector};
use lockstep_progression::{
    award, enrol, refund, unlock, Graph, NodeId, Owners, Progress, ProgressEvent, Refusal,
};

pub const REGISTRY: &str = r#"{
  "attributes": [
    { "name": "experience", "minimum": 0, "maximum": 100, "starting": 0 },
    { "name": "repair", "minimum": 0, "maximum": 100, "starting": 50 }
  ]
}"#;

/// Six nodes: basics, then electrical or mechanical (one excludes the other), diagnostics,
/// senior behind an experience gate, and chief, a keystone.
pub const GRAPH: &str = r#"{
  "refund_percent": 50,
  "triggers": [ { "name": "shift", "points": 1 }, { "name": "incident", "points": 3 } ],
  "nodes": [
    { "name": "basics", "cost": 1 },
    { "name": "electrical", "cost": 2, "requires": ["basics"], "excludes": ["mechanical"],
      "modifiers": [ { "attribute": "repair", "modifier": { "add": 10 } } ] },
    { "name": "mechanical", "cost": 2, "requires": ["basics"] },
    { "name": "diagnostics", "cost": 3, "requires": ["basics"], "refund_percent": 100 },
    { "name": "senior", "cost": 3, "requires": ["diagnostics"],
      "gates": [ { "attribute": "experience", "at_least": 50 } ] },
    { "name": "chief", "cost": 4, "requires": ["senior"], "keystone": true,
      "gates": [ { "attribute": "experience", "at_least": "80" } ] }
  ]
}"#;

pub fn whole(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

/// One technician with attributes, enrolled, and points to spend.
pub struct Crew {
    pub registry: Registry,
    pub graph: Graph,
    pub progress: Column<Progress>,
    pub attributes: Column<Attributes>,
    pub attribute_events: Vec<AttributeEvent>,
    pub events: Vec<ProgressEvent>,
    pub who: Handle,
}

impl Crew {
    pub fn new(points: u32) -> Self {
        let registry = Registry::from_json(REGISTRY).expect("valid registry");
        let graph = Graph::from_json(GRAPH, &registry).expect("valid graph");
        let who = StableVector::new().insert(());
        let mut progress = Column::new();
        enrol(&mut progress, who);
        progress.get_mut(who).unwrap().points = points;
        let mut attributes = Column::new();
        attributes.set(who, Attributes::from_registry(&registry));
        Crew {
            registry,
            graph,
            progress,
            attributes,
            attribute_events: Vec::new(),
            events: Vec::new(),
            who,
        }
    }

    pub fn node(&self, name: &str) -> NodeId {
        self.graph.node_id(name).expect("known node")
    }

    pub fn unlock(&mut self, name: &str) -> Result<(), Refusal> {
        let node = self.node(name);
        let mut owners = Owners {
            attributes: &mut self.attributes,
            registry: &self.registry,
            events: &mut self.attribute_events,
        };
        unlock(
            &mut self.progress,
            self.who,
            node,
            &self.graph,
            &mut owners,
            &mut self.events,
        )
    }

    pub fn refund(&mut self, name: &str) -> Result<u32, Refusal> {
        let node = self.node(name);
        let mut owners = Owners {
            attributes: &mut self.attributes,
            registry: &self.registry,
            events: &mut self.attribute_events,
        };
        refund(
            &mut self.progress,
            self.who,
            node,
            &self.graph,
            &mut owners,
            &mut self.events,
        )
    }

    pub fn award(&mut self, trigger: &str) -> Result<(), Refusal> {
        let trigger = self.graph.trigger_id(trigger).expect("known trigger");
        award(
            &mut self.progress,
            self.who,
            trigger,
            &self.graph,
            &mut self.events,
        )
    }

    pub fn points(&self) -> u32 {
        self.progress.get(self.who).unwrap().points
    }

    pub fn set(&mut self, attribute: &str, value: i32) {
        let id = self.registry.id(attribute).expect("known attribute");
        let attributes = self.attributes.get_mut(self.who).unwrap();
        let delta = whole(value) - attributes.get(id).current();
        attributes.apply(
            self.who,
            id,
            delta,
            &self.registry,
            &mut self.attribute_events,
        );
    }

    pub fn value(&self, attribute: &str) -> (Fixed32, Fixed32) {
        let id = self.registry.id(attribute).expect("known attribute");
        let value = self.attributes.get(self.who).unwrap().get(id);
        (value.current(), value.maximum())
    }
}
