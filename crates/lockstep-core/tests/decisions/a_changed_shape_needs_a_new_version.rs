// SPDX-License-Identifier: Apache-2.0
//! Decision 0062 and 0063: `#[derive(Message)]` writes the type's name, version and shape as JSON
//! into `Message::SCHEMA`, from the field types as written, and a saved schema per version is
//! compared with it, so a shape that changes without a new version fails a test.
//! Alternatives rejected: a hash of the shape (a failure says nothing about what changed), and
//! reading the schema from serde at run time (serde does not know the field types).
//! Would change if: two different shapes under one version give the same text, or the same shape
//! gives different text from one build to the next.

use lockstep_core::{Handle, Message};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

mod before {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
    #[message(version = 1)]
    pub struct Account {
        pub balance: i64,
    }
}

mod after {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
    #[message(version = 1)]
    pub struct Account {
        pub balance: i64,
        pub owner: u32,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 7)]
enum Order {
    Placed {
        buyer: Handle,
        lines: Vec<Option<(u8, [u16; 2])>>,
    },
    Cancelled(Handle),
    Closed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 3)]
struct Pair<T>(T, Box<T>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 2)]
struct Nothing;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Message)]
#[message(version = 4)]
struct Table<const N: usize>(std::collections::BTreeMap<u8, String>, [u8; 4]);

#[test]
fn a_shape_changed_under_the_same_version_gives_a_different_schema() {
    assert_eq!(
        <before::Account as Message>::VERSION,
        <after::Account as Message>::VERSION
    );
    assert_ne!(
        <before::Account as Message>::SCHEMA,
        <after::Account as Message>::SCHEMA
    );
}

#[test]
fn a_struct_schema_names_its_version_and_fields() {
    assert_eq!(
        <before::Account as Message>::SCHEMA,
        r#"{
  "name": "Account",
  "version": 1,
  "kind": "struct",
  "form": "named",
  "fields": [
    { "name": "balance", "type": "i64" }
  ]
}"#
    );
    assert_eq!(
        <Nothing as Message>::SCHEMA,
        r#"{
  "name": "Nothing",
  "version": 2,
  "kind": "struct",
  "form": "unit",
  "fields": []
}"#
    );
}

#[test]
fn an_enum_schema_lists_its_variants_in_declaration_order_with_types_as_written() {
    assert_eq!(
        <Order as Message>::SCHEMA,
        r#"{
  "name": "Order",
  "version": 7,
  "kind": "enum",
  "variants": [
    {
      "index": 0,
      "name": "Placed",
      "form": "named",
      "fields": [
        { "name": "buyer", "type": "Handle" },
        { "name": "lines", "type": "Vec<Option<(u8,[u16;2])>>" }
      ]
    },
    {
      "index": 1,
      "name": "Cancelled",
      "form": "tuple",
      "fields": [
        { "name": "0", "type": "Handle" }
      ]
    },
    {
      "index": 2,
      "name": "Closed",
      "form": "unit",
      "fields": []
    }
  ]
}"#
    );
}

#[test]
fn a_generic_message_lists_its_parameters() {
    assert_eq!(
        <Pair<u8> as Message>::SCHEMA,
        r#"{
  "name": "Pair",
  "version": 3,
  "generics": [
    "T"
  ],
  "kind": "struct",
  "form": "tuple",
  "fields": [
    { "name": "0", "type": "T" },
    { "name": "1", "type": "Box<T>" }
  ]
}"#
    );
}

#[test]
fn paths_and_constant_parameters_keep_their_spelling() {
    assert_eq!(
        <Table<2> as Message>::SCHEMA,
        r#"{
  "name": "Table",
  "version": 4,
  "generics": [
    "N"
  ],
  "kind": "struct",
  "form": "tuple",
  "fields": [
    { "name": "0", "type": "std::collections::BTreeMap<u8,String>" },
    { "name": "1", "type": "[u8;4]" }
  ]
}"#
    );
}
