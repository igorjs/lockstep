// SPDX-License-Identifier: Apache-2.0
use crate::common::{whole, GRAPH, REGISTRY};
use lockstep_attributes::{Modifier, Registry};
use lockstep_progression::{Graph, GraphError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn registry() -> Registry {
    Registry::from_json(REGISTRY).unwrap()
}

#[test]
fn a_graph_reads_nodes_triggers_gates_and_both_sides_of_an_exclusion() {
    let registry = registry();
    let graph = Graph::from_json(GRAPH, &registry).unwrap();
    assert_eq!(graph.nodes.len(), 6);
    let id = |name| graph.node_id(name).unwrap();
    let electrical = graph.node(id("electrical")).unwrap();
    assert_eq!(electrical.excludes, vec![id("mechanical")]);
    assert_eq!(
        graph.node(id("mechanical")).unwrap().excludes,
        vec![id("electrical")]
    );
    assert_eq!(electrical.refund_percent, 50, "the graph's default");
    assert_eq!(graph.node(id("diagnostics")).unwrap().refund_percent, 100);
    assert_eq!(
        electrical.modifiers,
        vec![(registry.id("repair").unwrap(), Modifier::Add(whole(10)))]
    );
    let chief = graph.node(id("chief")).unwrap();
    assert!(chief.keystone);
    assert_eq!(chief.gates[0].at_least, whole(80));
    assert_eq!(graph.triggers[1].points, 3);
}

#[test]
fn a_bad_graph_is_refused_with_its_reason() {
    let registry = registry();
    let read = |nodes: &str| Graph::from_json(&format!(r#"{{ "nodes": [ {nodes} ] }}"#), &registry);
    let cases = [
        (
            r#"{ "name": "a", "cost": 1 }, { "name": "a", "cost": 1 }"#,
            GraphError::DuplicateName("a".into()),
        ),
        (
            r#"{ "name": "a", "cost": 1, "requires": ["b"] }"#,
            GraphError::UnknownNode {
                owner: "a".into(),
                node: "b".into(),
            },
        ),
        (
            r#"{ "name": "a", "cost": 1, "gates": [ { "attribute": "luck", "at_least": 1 } ] }"#,
            GraphError::UnknownAttribute {
                owner: "a".into(),
                attribute: "luck".into(),
            },
        ),
        (
            r#"{ "name": "a", "cost": 1, "requires": ["a"] }"#,
            GraphError::Contradiction("a".into()),
        ),
        (
            r#"{ "name": "a", "cost": 1 }, { "name": "b", "cost": 1, "requires": ["a"], "excludes": ["a"] }"#,
            GraphError::Contradiction("b".into()),
        ),
        // An exclusion written on the other node still contradicts a requirement.
        (
            r#"{ "name": "a", "cost": 1, "excludes": ["b"] }, { "name": "b", "cost": 1, "requires": ["a"] }"#,
            GraphError::Contradiction("b".into()),
        ),
        (
            r#"{ "name": "a", "cost": 1, "requires": ["b"] }, { "name": "b", "cost": 1, "requires": ["a"] }"#,
            GraphError::Cycle("a".into()),
        ),
        (
            r#"{ "name": "a", "cost": 1, "refund_percent": 101 }"#,
            GraphError::RefundOver100("a".into()),
        ),
        // Requiring both sides of an exclusion: no owner could ever take it.
        (
            r#"{ "name": "a", "cost": 1, "excludes": ["b"] }, { "name": "b", "cost": 1 }, { "name": "c", "cost": 1, "requires": ["a", "b"] }"#,
            GraphError::Contradiction("c".into()),
        ),
        // Excluding something two links down its own chain.
        (
            r#"{ "name": "a", "cost": 1 }, { "name": "b", "cost": 1, "requires": ["a"] }, { "name": "c", "cost": 1, "requires": ["b"], "excludes": ["a"] }"#,
            GraphError::Contradiction("c".into()),
        ),
    ];
    for (nodes, error) in cases {
        assert_eq!(read(nodes), Err(error), "{nodes}");
    }
}

#[test]
fn a_long_requirement_chain_loads_without_running_out_of_stack() {
    let registry = registry();
    let nodes: Vec<String> = (0..20_000)
        .map(|index| {
            if index == 0 {
                r#"{ "name": "n0", "cost": 1 }"#.to_string()
            } else {
                format!(
                    r#"{{ "name": "n{index}", "cost": 1, "requires": ["n{}"] }}"#,
                    index - 1
                )
            }
        })
        .collect();
    let text = format!(r#"{{ "nodes": [ {} ] }}"#, nodes.join(", "));
    assert_eq!(
        Graph::from_json(&text, &registry).map(|graph| graph.nodes.len()),
        Ok(20_000)
    );
}
