// SPDX-License-Identifier: Apache-2.0
//! Decision: messages are derived with a required version, and the same derive writes the
//! timeline index: the kind is the variant index and the handles are found from the field types.
//! The derive only declares facts, so it never changes how a value serializes.
//! Alternative rejected: a default version when the attribute is missing (a changed shape would
//! ship under the old number), and a separate index derive that must agree with this one.
//! Would change if: a derived message serializes differently from the same shape with a hand
//! written `Message`, or the kinds stop being the variant indexes in declaration order.

use lockstep_core::{hash_of, Handle, Indexable, Message, StableVector};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 5)]
enum Derived {
    Opened {
        account: Handle,
    },
    Moved {
        from: Handle,
        to: Handle,
        amount: i64,
    },
    Closed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum ByHand {
    Opened {
        account: Handle,
    },
    Moved {
        from: Handle,
        to: Handle,
        amount: i64,
    },
    Closed,
}

impl Message for ByHand {
    const VERSION: u32 = 5;
}

#[test]
fn a_derived_message_serializes_exactly_like_one_written_by_hand() {
    let mut store = StableVector::new();
    let (a, b) = (store.insert(()), store.insert(()));
    let derived = [
        Derived::Opened { account: a },
        Derived::Moved {
            from: a,
            to: b,
            amount: -3,
        },
        Derived::Closed,
    ];
    let by_hand = [
        ByHand::Opened { account: a },
        ByHand::Moved {
            from: a,
            to: b,
            amount: -3,
        },
        ByHand::Closed,
    ];
    assert_eq!(hash_of(&derived), hash_of(&by_hand));
    assert_eq!(Derived::VERSION, ByHand::VERSION);
}

#[test]
fn kinds_are_variant_indexes_in_declaration_order() {
    let mut store = StableVector::new();
    let (a, b) = (store.insert(()), store.insert(()));
    let kinds: Vec<u16> = [
        Derived::Opened { account: a },
        Derived::Moved {
            from: a,
            to: b,
            amount: 1,
        },
        Derived::Closed,
    ]
    .iter()
    .map(Indexable::kind)
    .collect();
    assert_eq!(kinds, [0, 1, 2]);
}
