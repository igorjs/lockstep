// SPDX-License-Identifier: Apache-2.0
//! Behaviour tests for templates, pools, reserved chunks and applying a layout.

use lockstep_procgen::{Reservation, RollError, Templates, TemplatesError};
use lockstep_spatial::{GridMap, Square4};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const TEMPLATES: &str = r###"{
  "side": 2,
  "templates": [
    { "name": "meadow", "rows": ["..", ".."] },
    { "name": "rocks", "rows": ["#.", ".#"] },
    { "name": "marsh", "rows": ["~~", "~~"] },
    { "name": "keep", "rows": ["##", "##"] }
  ],
  "pools": [
    { "name": "wild", "entries": [ { "template": "meadow", "weight": 4 },
                                   { "template": "rocks", "weight": 1 } ] },
    { "name": "wet", "entries": [ { "template": "marsh", "weight": 1 } ] }
  ]
}"###;

fn templates() -> Templates {
    Templates::from_json(TEMPLATES).unwrap()
}

#[test]
fn a_roll_is_repeatable_and_follows_the_seed() {
    let templates = templates();
    let wild = templates.pool_id("wild").unwrap();
    let first = templates.roll(5, 8, 8, wild, &[]).unwrap();
    assert_eq!(first, templates.roll(5, 8, 8, wild, &[]).unwrap());
    assert_ne!(first, templates.roll(6, 8, 8, wild, &[]).unwrap());
}

#[test]
fn only_templates_of_the_pool_are_drawn_and_weights_are_followed() {
    let templates = templates();
    let wild = templates.pool_id("wild").unwrap();
    let meadow = templates.template_id("meadow").unwrap();
    let rocks = templates.template_id("rocks").unwrap();
    let layout = templates.roll(11, 100, 100, wild, &[]).unwrap();
    let mut meadows = 0;
    for y in 0..100 {
        for x in 0..100 {
            let drawn = layout.template_at(x, y).unwrap();
            assert!(drawn == meadow || drawn == rocks);
            meadows += (drawn == meadow) as u32;
        }
    }
    // 4 of 5 is 8,000 of 10,000.
    assert!(meadows.abs_diff(8_000) <= 300, "{meadows}");
}

#[test]
fn reserved_chunks_hold_their_template_and_leave_the_rest_alone() {
    let templates = templates();
    let wild = templates.pool_id("wild").unwrap();
    let keep = templates.template_id("keep").unwrap();
    let reserved = [Reservation {
        x: 2,
        y: 1,
        template: keep,
    }];
    let free = templates.roll(3, 5, 5, wild, &[]).unwrap();
    let held = templates.roll(3, 5, 5, wild, &reserved).unwrap();
    assert_eq!(held.template_at(2, 1), Some(keep));
    for y in 0..5 {
        for x in 0..5 {
            if (x, y) != (2, 1) {
                assert_eq!(held.template_at(x, y), free.template_at(x, y));
            }
        }
    }
}

#[test]
fn rolls_are_refused_with_a_reason() {
    let templates = templates();
    let wild = templates.pool_id("wild").unwrap();
    let keep = templates.template_id("keep").unwrap();
    let at = |x, y| Reservation {
        x,
        y,
        template: keep,
    };
    assert_eq!(templates.roll(1, 0, 3, wild, &[]), Err(RollError::Empty));
    assert_eq!(
        templates.roll(1, 3, 3, wild, &[at(3, 0)]),
        Err(RollError::BadReservation(0))
    );
    assert_eq!(
        templates.roll(1, 3, 3, wild, &[at(1, 1), at(1, 1)]),
        Err(RollError::Reserved(1))
    );
    assert_eq!(
        templates.roll(1, 3, 3, lockstep_procgen::PoolId(9), &[]),
        Err(RollError::UnknownPool)
    );
}

#[test]
fn applying_writes_walls_marsh_and_marks_chunks_dirty() {
    let templates = templates();
    let wet = templates.pool_id("wet").unwrap();
    let keep = templates.template_id("keep").unwrap();
    let layout = templates
        .roll(
            1,
            2,
            1,
            wet,
            &[Reservation {
                x: 1,
                y: 0,
                template: keep,
            }],
        )
        .unwrap();
    let mut map: GridMap<Square4> = GridMap::new(4, 2);
    map.take_dirty_chunks();
    layout.apply(&templates, &mut map).unwrap();
    assert!(map.is_passable(map.index(0, 0)) && map.cost(map.index(0, 0)) == 3);
    assert!(!map.is_passable(map.index(3, 1)));
    assert_eq!(map.take_dirty_chunks(), vec![0]);
    let mut small: GridMap<Square4> = GridMap::new(3, 2);
    assert_eq!(
        layout.apply(&templates, &mut small),
        Err(RollError::MapTooSmall)
    );
}

#[test]
fn bad_data_is_refused() {
    let bad = |text: &str| Templates::from_json(text).unwrap_err();
    let body = |side: u32, rows: &str, pool: &str| {
        format!(
            r#"{{ "side": {side}, "templates": [ {{ "name": "a", "rows": {rows} }} ],
                 "pools": [ {pool} ] }}"#
        )
    };
    let pool = r#"{ "name": "p", "entries": [ { "template": "a", "weight": 1 } ] }"#;
    assert!(matches!(bad("{"), TemplatesError::Json(_)));
    assert_eq!(bad(&body(0, r#"[]"#, pool)), TemplatesError::BadSide(0));
    assert_eq!(
        bad(&body(2, r#"["..", "."]"#, pool)),
        TemplatesError::BadShape("a".into())
    );
    assert_eq!(
        bad(&body(1, r#"["x"]"#, pool)),
        TemplatesError::UnknownGlyph {
            template: "a".into(),
            glyph: 'x'
        }
    );
    assert_eq!(
        bad(&body(
            1,
            r#"["."]"#,
            r#"{ "name": "p", "entries": [ { "template": "b", "weight": 1 } ] }"#
        )),
        TemplatesError::UnknownTemplate {
            pool: "p".into(),
            template: "b".into()
        }
    );
    assert_eq!(
        bad(&body(
            1,
            r#"["."]"#,
            r#"{ "name": "p", "entries": [ { "template": "a", "weight": 0 } ] }"#
        )),
        TemplatesError::Weights("p".into())
    );
}
