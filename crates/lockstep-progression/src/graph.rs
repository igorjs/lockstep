// SPDX-License-Identifier: Apache-2.0
use lockstep_attributes::{AttributeId, Modifier, Registry};
use lockstep_core::math::Fixed32;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct NodeId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TriggerId(pub u16);

/// A behavioural gate: an attribute at or above a value when the node is taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    pub attribute: AttributeId,
    pub at_least: Fixed32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub cost: u32,
    /// Every one of these must be taken first.
    pub requires: Vec<NodeId>,
    /// Taking either of a pair locks the other. Kept on both nodes.
    pub excludes: Vec<NodeId>,
    pub gates: Vec<Gate>,
    /// A keystone can never be refunded.
    pub keystone: bool,
    /// Percent of the cost a refund returns.
    pub refund_percent: u32,
    /// Applied to the owner's attributes while the node is taken.
    pub modifiers: Vec<(AttributeId, Modifier)>,
}

/// Something that happens and earns points, such as a completed shift.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub points: u32,
}

/// Every node and trigger, in a stable order. Build it with `Graph::from_json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub triggers: Vec<Trigger>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    Json(String),
    TooMany,
    DuplicateName(String),
    UnknownNode {
        owner: String,
        node: String,
    },
    UnknownAttribute {
        owner: String,
        attribute: String,
    },
    /// A node requiring or excluding itself, or requiring a node it excludes.
    Contradiction(String),
    /// Requirements that loop can never be met. Names a node on the loop.
    Cycle(String),
    RefundOver100(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GraphError {}

impl Graph {
    /// Reads the data form documented in `docs/lockstep-progression.md`. Gates and modifiers name
    /// attributes in `attributes`.
    pub fn from_json(text: &str, attributes: &Registry) -> Result<Self, GraphError> {
        let file: file::GraphFile =
            serde_json::from_str(text).map_err(|error| GraphError::Json(error.to_string()))?;
        file.into_graph(attributes)
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize)
    }

    pub fn node_id(&self, name: &str) -> Option<NodeId> {
        position(self.nodes.iter().map(|node| node.name.as_str()), name).map(NodeId)
    }

    pub fn trigger_id(&self, name: &str) -> Option<TriggerId> {
        position(
            self.triggers.iter().map(|trigger| trigger.name.as_str()),
            name,
        )
        .map(TriggerId)
    }

    /// Refuses requirements that loop, and nodes no owner could ever take: a node whose full set
    /// of requirements (theirs included) holds two nodes that exclude each other, or a node it
    /// excludes. Iterative, so a long chain cannot overflow the stack.
    fn check(&self) -> Result<(), GraphError> {
        let count = self.nodes.len();
        // Kahn's order: a node comes after everything it requires.
        let mut waiting: Vec<usize> = self.nodes.iter().map(|node| node.requires.len()).collect();
        let mut required_by = vec![Vec::new(); count];
        for (index, node) in self.nodes.iter().enumerate() {
            for required in &node.requires {
                required_by[required.0 as usize].push(index);
            }
        }
        let mut ready: Vec<usize> = (0..count)
            .rev()
            .filter(|index| waiting[*index] == 0)
            .collect();
        let mut order = Vec::with_capacity(count);
        while let Some(index) = ready.pop() {
            order.push(index);
            for dependant in required_by[index].iter().rev() {
                waiting[*dependant] -= 1;
                if waiting[*dependant] == 0 {
                    ready.push(*dependant);
                }
            }
        }
        if order.len() < count {
            let looped = (0..count)
                .find(|index| waiting[*index] > 0)
                .expect("a node left over");
            return Err(GraphError::Cycle(self.nodes[looped].name.clone()));
        }
        // For each node that takes part in an exclusion, every node that requires it, directly or
        // not: a walk forward over `required_by`. Graphs have few exclusions, so this stays
        // linear in the nodes for each one.
        let mut excluded_nodes: BTreeSet<usize> = BTreeSet::new();
        for node in &self.nodes {
            excluded_nodes.extend(node.excludes.iter().map(|excluded| excluded.0 as usize));
        }
        let mut needs: BTreeMap<usize, Vec<bool>> = BTreeMap::new();
        for start in excluded_nodes {
            let mut reached = vec![false; count];
            let mut stack = required_by[start].clone();
            while let Some(index) = stack.pop() {
                if !reached[index] {
                    reached[index] = true;
                    stack.extend(required_by[index].iter().copied());
                }
            }
            needs.insert(start, reached);
        }
        let requires = |node: usize, of: usize| needs.get(&of).is_some_and(|reached| reached[node]);
        let excluders: Vec<usize> = (0..count)
            .filter(|index| !self.nodes[*index].excludes.is_empty())
            .collect();
        for (index, node) in self.nodes.iter().enumerate() {
            // A node excluding something it needs, or needing both sides of an exclusion.
            let clash = node
                .excludes
                .iter()
                .any(|excluded| requires(index, excluded.0 as usize))
                || excluders.iter().any(|other| {
                    requires(index, *other)
                        && self.nodes[*other]
                            .excludes
                            .iter()
                            .any(|excluded| requires(index, excluded.0 as usize))
                });
            if clash {
                return Err(GraphError::Contradiction(node.name.clone()));
            }
        }
        Ok(())
    }
}

fn position<'a>(names: impl Iterator<Item = &'a str>, name: &str) -> Option<u16> {
    names
        .enumerate()
        .find(|(_, known)| *known == name)
        .map(|(index, _)| index as u16)
}

mod file {
    use super::*;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct GraphFile {
        /// The refund percent for nodes that do not set their own.
        #[serde(default)]
        refund_percent: u32,
        #[serde(default)]
        triggers: Vec<Trigger>,
        nodes: Vec<NodeFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct NodeFile {
        name: String,
        cost: u32,
        #[serde(default)]
        requires: Vec<String>,
        #[serde(default)]
        excludes: Vec<String>,
        #[serde(default)]
        gates: Vec<GateFile>,
        #[serde(default)]
        keystone: bool,
        #[serde(default)]
        refund_percent: Option<u32>,
        #[serde(default)]
        modifiers: Vec<ModifierFile>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GateFile {
        attribute: String,
        at_least: Decimal,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ModifierFile {
        attribute: String,
        modifier: ModifierKindFile,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    enum ModifierKindFile {
        Add(Decimal),
        Multiply(Decimal),
        Override(Decimal),
    }

    #[derive(Deserialize)]
    #[serde(try_from = "DecimalFile")]
    struct Decimal(Fixed32);

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DecimalFile {
        Text(String),
        Whole(i64),
    }

    impl TryFrom<DecimalFile> for Decimal {
        type Error = String;
        fn try_from(value: DecimalFile) -> Result<Self, String> {
            match value {
                DecimalFile::Text(text) => Fixed32::parse_decimal(&text)
                    .map(Decimal)
                    .map_err(|error| format!("{text:?}: {error}")),
                DecimalFile::Whole(whole) => i32::try_from(whole)
                    .ok()
                    .filter(|whole| (-32_768..=32_767).contains(whole))
                    .map(|whole| Decimal(Fixed32::from_int(whole)))
                    .ok_or_else(|| format!("{whole} is outside the 16.16 range")),
            }
        }
    }

    impl GraphFile {
        pub(super) fn into_graph(self, attributes: &Registry) -> Result<Graph, GraphError> {
            if self.nodes.len() > u16::MAX as usize || self.triggers.len() > u16::MAX as usize {
                return Err(GraphError::TooMany);
            }
            for names in [
                self.nodes
                    .iter()
                    .map(|node| node.name.as_str())
                    .collect::<Vec<_>>(),
                self.triggers
                    .iter()
                    .map(|trigger| trigger.name.as_str())
                    .collect(),
            ] {
                let mut seen = BTreeSet::new();
                if let Some(repeat) = names.into_iter().find(|name| !seen.insert(*name)) {
                    return Err(GraphError::DuplicateName(repeat.to_string()));
                }
            }
            let names: BTreeMap<String, u16> = self
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.name.clone(), index as u16))
                .collect();
            let node = |owner: &str, name: &str| {
                names
                    .get(name)
                    .copied()
                    .map(NodeId)
                    .ok_or_else(|| GraphError::UnknownNode {
                        owner: owner.to_string(),
                        node: name.to_string(),
                    })
            };
            let attribute = |owner: &str, name: &str| {
                attributes
                    .id(name)
                    .ok_or_else(|| GraphError::UnknownAttribute {
                        owner: owner.to_string(),
                        attribute: name.to_string(),
                    })
            };
            let mut nodes = Vec::new();
            for (index, file) in self.nodes.into_iter().enumerate() {
                let requires = file
                    .requires
                    .iter()
                    .map(|name| node(&file.name, name))
                    .collect::<Result<Vec<_>, _>>()?;
                let excludes = file
                    .excludes
                    .iter()
                    .map(|name| node(&file.name, name))
                    .collect::<Result<Vec<_>, _>>()?;
                let itself = NodeId(index as u16);
                if requires.contains(&itself)
                    || excludes.contains(&itself)
                    || requires.iter().any(|required| excludes.contains(required))
                {
                    return Err(GraphError::Contradiction(file.name));
                }
                let refund_percent = file.refund_percent.unwrap_or(self.refund_percent);
                if refund_percent > 100 {
                    return Err(GraphError::RefundOver100(file.name));
                }
                let gates = file
                    .gates
                    .iter()
                    .map(|gate| {
                        Ok(Gate {
                            attribute: attribute(&file.name, &gate.attribute)?,
                            at_least: gate.at_least.0,
                        })
                    })
                    .collect::<Result<Vec<_>, GraphError>>()?;
                let modifiers = file
                    .modifiers
                    .iter()
                    .map(|modifier| {
                        let value = match &modifier.modifier {
                            ModifierKindFile::Add(value) => Modifier::Add(value.0),
                            ModifierKindFile::Multiply(value) => Modifier::Multiply(value.0),
                            ModifierKindFile::Override(value) => Modifier::Override(value.0),
                        };
                        Ok((attribute(&file.name, &modifier.attribute)?, value))
                    })
                    .collect::<Result<Vec<_>, GraphError>>()?;
                nodes.push(Node {
                    name: file.name,
                    cost: file.cost,
                    requires,
                    excludes,
                    gates,
                    keystone: file.keystone,
                    refund_percent,
                    modifiers,
                });
            }
            // Exclusions go both ways: add each to the other side, once, in order.
            for index in 0..nodes.len() {
                for excluded in nodes[index].excludes.clone() {
                    let other = &mut nodes[excluded.0 as usize].excludes;
                    if !other.contains(&NodeId(index as u16)) {
                        other.push(NodeId(index as u16));
                    }
                }
            }
            for node in &mut nodes {
                node.excludes.sort();
            }
            let graph = Graph {
                nodes,
                triggers: self.triggers,
            };
            graph.check()?;
            Ok(graph)
        }
    }
}
