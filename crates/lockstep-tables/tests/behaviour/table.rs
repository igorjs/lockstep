// SPDX-License-Identifier: Apache-2.0
use crate::common::{Size, BROKEN, LARGE, SMALL};
use lockstep_core::{decode, encode};
use lockstep_tables::{Swapped, Table, TableError, MAXIMUM_ROWS, MAXIMUM_SWAP_BYTES};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn two_rows(first: &str, second: &str) -> String {
    format!(r#"{{"rows":[{{"name":"{first}","amount":1}},{{"name":"{second}","amount":2}}]}}"#)
}

#[test]
fn rows_are_read_by_name() {
    let table = Table::<Size>::from_json(&two_rows("b", "a")).unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table.get("a"), Some(&Size { amount: 2 }));
    assert_eq!(table.get("missing"), None);
    let names: Vec<&str> = table.iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["a", "b"], "name order, not file order");
}

#[test]
fn the_order_of_the_file_does_not_change_the_digest() {
    let one =
        Table::<Size>::from_json(r#"{"rows":[{"name":"a","amount":2},{"name":"b","amount":1}]}"#)
            .unwrap();
    let other =
        Table::<Size>::from_json(r#"{"rows":[{"name":"b","amount":1},{"name":"a","amount":2}]}"#)
            .unwrap();
    assert_eq!(one.digest(), other.digest());
    assert_ne!(
        one.digest(),
        Table::<Size>::from_json(&two_rows("a", "b"))
            .unwrap()
            .digest()
    );
}

#[test]
fn a_table_is_refused_for_each_fault() {
    assert!(matches!(
        Table::<Size>::from_json("{"),
        Err(TableError::Json(_))
    ));
    assert_eq!(
        Table::<Size>::from_json(r#"{"rows":[]}"#),
        Err(TableError::Empty)
    );
    assert_eq!(
        Table::<Size>::from_json(&two_rows("a", "a")),
        Err(TableError::DuplicateName("a".into()))
    );
    assert_eq!(
        Table::<Size>::from_json(BROKEN),
        Err(TableError::InvalidRow {
            name: "step".into(),
            reason: "a size below zero".into()
        })
    );
    assert!(matches!(
        Table::<Size>::from_json(r#"{"rows":[{"name":"a"}]}"#),
        Err(TableError::Json(_))
    ));
}

#[test]
fn a_table_has_a_bounded_size() {
    let text = format!(
        r#"{{"rows":[],"padding":"{}"}}"#,
        "x".repeat(MAXIMUM_SWAP_BYTES)
    );
    assert_eq!(Table::<Size>::from_json(&text), Err(TableError::TooLarge));
    // Past the row limit, a decoded table is refused too (the text limit stops JSON first).
    let rows: std::collections::BTreeMap<String, Size> = (0..=MAXIMUM_ROWS)
        .map(|index| (index.to_string(), Size { amount: 0 }))
        .collect();
    assert!(decode::<Table<Size>>(&encode(&rows)).is_err());
}

#[test]
fn swapping_replaces_reports_and_ignores_the_same_content() {
    let mut table = Table::<Size>::from_json(SMALL).unwrap();
    let from = table.digest();
    let Ok(Swapped::Replaced {
        from: was,
        to,
        rows,
    }) = table.swap(LARGE)
    else {
        panic!("expected a replacement");
    };
    assert_eq!((was, rows), (from, 1));
    assert_eq!(to, table.digest());
    assert_ne!(from, to);
    assert_eq!(table.get("step"), Some(&Size { amount: 10 }));
    assert_eq!(table.swap(LARGE), Ok(Swapped::Unchanged));
}

#[test]
fn a_failed_swap_leaves_the_table_as_it_was() {
    let mut table = Table::<Size>::from_json(SMALL).unwrap();
    let before = table.clone();
    assert!(table.swap(BROKEN).is_err());
    assert!(table.swap("nonsense").is_err());
    assert_eq!(table, before);
}

#[test]
fn decoding_runs_the_same_checks_as_reading() {
    let table = Table::<Size>::from_json(SMALL).unwrap();
    let bytes = encode(&table);
    assert_eq!(decode::<Table<Size>>(&bytes).unwrap(), table);
    // The same bytes with the amount below zero: a damaged save cannot carry it.
    let broken = encode(&std::collections::BTreeMap::from([(
        "step".to_string(),
        Size { amount: -5 },
    )]));
    assert!(decode::<Table<Size>>(&broken).is_err());
    let empty = encode(&std::collections::BTreeMap::<String, Size>::new());
    assert!(decode::<Table<Size>>(&empty).is_err());
}
