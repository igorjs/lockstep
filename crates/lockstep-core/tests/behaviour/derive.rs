// SPDX-License-Identifier: Apache-2.0
use lockstep_core::{Handle, Indexable, Message, StableVector};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 3)]
struct Snapshot {
    owner: Handle,
    count: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
enum Happened {
    Nothing,
    Moved {
        entity: Handle,
        distance: u32,
    },
    Traded(Handle, Handle, i64),
    Joined {
        member: Option<Handle>,
        crew: Vec<Handle>,
        label: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 7)]
struct Pair<T> {
    first: T,
    second: T,
}

fn handles(value: &impl Indexable) -> Vec<Handle> {
    let mut out = Vec::new();
    value.handles(&mut out);
    out
}

#[test]
fn the_derive_writes_the_version_from_the_attribute() {
    assert_eq!(Snapshot::VERSION, 3);
    assert_eq!(Happened::VERSION, 1);
    assert_eq!(Pair::<u8>::VERSION, 7);
}

#[test]
fn the_kind_is_the_variant_index_and_zero_for_a_struct() {
    let mut store = StableVector::new();
    let (a, b) = (store.insert(()), store.insert(()));
    assert_eq!(Snapshot { owner: a, count: 1 }.kind(), 0);
    assert_eq!(Happened::Nothing.kind(), 0);
    assert_eq!(
        Happened::Moved {
            entity: a,
            distance: 3
        }
        .kind(),
        1
    );
    assert_eq!(Happened::Traded(a, b, 5).kind(), 2);
    assert_eq!(
        Happened::Joined {
            member: None,
            crew: vec![],
            label: String::new()
        }
        .kind(),
        3
    );
}

#[test]
fn the_handles_are_every_handle_field_in_field_order() {
    let mut store = StableVector::new();
    let (a, b, c) = (store.insert(()), store.insert(()), store.insert(()));
    assert_eq!(handles(&Snapshot { owner: b, count: 9 }), [b]);
    assert_eq!(handles(&Happened::Nothing), []);
    assert_eq!(
        handles(&Happened::Moved {
            entity: c,
            distance: 1
        }),
        [c]
    );
    assert_eq!(handles(&Happened::Traded(b, a, -4)), [b, a]);
    let joined = Happened::Joined {
        member: Some(c),
        crew: vec![a, b],
        label: "night".into(),
    };
    assert_eq!(handles(&joined), [c, a, b]);
    let alone = Happened::Joined {
        member: None,
        crew: vec![],
        label: String::new(),
    };
    assert_eq!(handles(&alone), []);
    // A generic field is not a handle, even when it holds one.
    assert_eq!(
        handles(&Pair {
            first: a,
            second: b
        }),
        []
    );
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 1)]
struct Shapes {
    pair: [Handle; 2],
    tuple: (u8, Handle, Option<Handle>),
    boxed: Box<Handle>,
    nested: Option<Vec<Handle>>,
    list_of_lists: Vec<Vec<Handle>>,
    unrelated: Vec<u32>,
}

#[test]
fn handles_are_found_inside_arrays_tuples_boxes_and_nested_containers() {
    let mut store = StableVector::new();
    let all: Vec<Handle> = (0..9).map(|_| store.insert(())).collect();
    let shapes = Shapes {
        pair: [all[0], all[1]],
        tuple: (7, all[2], Some(all[3])),
        boxed: Box::new(all[4]),
        nested: Some(vec![all[5], all[6]]),
        list_of_lists: vec![vec![all[7]], vec![], vec![all[8]]],
        unrelated: vec![1, 2, 3],
    };
    assert_eq!(handles(&shapes), all);
}

/// Handles kept in a map, which the derive cannot see, so the type writes `Indexable` itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 4, manual_indexable)]
struct Roster {
    members: std::collections::BTreeMap<u32, Handle>,
}

impl Indexable for Roster {
    fn kind(&self) -> u16 {
        9
    }

    fn handles(&self, out: &mut Vec<Handle>) {
        out.extend(self.members.values().copied());
    }
}

#[test]
fn manual_indexable_leaves_the_index_to_the_type() {
    let mut store = StableVector::new();
    let (a, b) = (store.insert(()), store.insert(()));
    let roster = Roster {
        members: [(2, b), (1, a)].into_iter().collect(),
    };
    assert_eq!(Roster::VERSION, 4);
    assert_eq!(roster.kind(), 9);
    assert_eq!(handles(&roster), [a, b]);
}
