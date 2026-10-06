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
