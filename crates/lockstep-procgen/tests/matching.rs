// SPDX-License-Identifier: Apache-2.0
//! Decision and behaviour tests for matching template edges (deferral D6, decision 0051).

use lockstep_procgen::{Reservation, RollError, Templates, TemplatesError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Every edge carries the one socket, so every pairing is allowed.
const FREE: &str = r###"{
  "side": 2,
  "templates": [
    { "name": "meadow", "rows": ["..", ".."],
      "sockets": { "north": "any", "east": "any", "south": "any", "west": "any" } },
    { "name": "rocks", "rows": ["..", ".."],
      "sockets": { "north": "any", "east": "any", "south": "any", "west": "any" } },
    { "name": "marsh", "rows": ["..", ".."],
      "sockets": { "north": "any", "east": "any", "south": "any", "west": "any" } }
  ],
  "pools": [
    { "name": "wild", "entries": [ { "template": "meadow", "weight": 4 },
                                   { "template": "rocks", "weight": 2 },
                                   { "template": "marsh", "weight": 1 } ] }
  ]
}"###;

/// The same templates with no sockets at all.
fn plain(text: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
    for template in value["templates"].as_array_mut().unwrap() {
        template.as_object_mut().unwrap().remove("sockets");
    }
    value.to_string()
}

/// North and south are always a post; east and west alternate between open and wall: a chain of
/// tiles that only alternate.
const ALTERNATING: &str = r###"{
  "side": 2,
  "templates": [
    { "name": "upright", "rows": ["..", ".."],
      "sockets": { "north": "post", "east": "open", "south": "post", "west": "wall" } },
    { "name": "sideways", "rows": ["..", ".."],
      "sockets": { "north": "post", "east": "wall", "south": "post", "west": "open" } }
  ],
  "pools": [
    { "name": "chain", "entries": [ { "template": "upright", "weight": 1 },
                                    { "template": "sideways", "weight": 1 } ] }
  ]
}"###;

/// Decision, the gate: matching is opt-in. A plain `roll` ignores sockets and draws exactly as it
/// did before sockets existed, so a template set without sockets and the same set with them roll
/// the same layout, and every committed fixture hash stays as it was.
/// Alternative rejected: matching inside `roll`, which would redraw every existing fixture.
/// Would change if: `roll` of a socketed set differs from `roll` of the same set without sockets.
#[test]
fn a_plain_roll_ignores_sockets() {
    let with = Templates::from_json(FREE).unwrap();
    let without = Templates::from_json(&plain(FREE)).unwrap();
    let wild = with.pool_id("wild").unwrap();
    assert!(with.has_sockets() && !without.has_sockets());
    assert_eq!(
        with.roll(7, 9, 9, wild, &[]).unwrap(),
        without
            .roll(7, 9, 9, without.pool_id("wild").unwrap(), &[])
            .unwrap()
    );
}

/// Decision, the gate: a matched roll draws from the same chunk seeds as a plain one, in the same
/// stream, and a chunk's first try is the plain draw. Where every pairing is allowed the matched
/// roll equals the plain one, in a small and in a large layout alike, so seeds never collide and
/// a chunk is the same in a bigger layout.
/// Alternative rejected: a stream of its own per layout walked in order, where one refused try
/// would move every chunk after it.
/// Would change if: the matched roll of a free set differs from the plain roll.
#[test]
fn a_matched_roll_with_nothing_in_the_way_is_the_plain_roll() {
    let templates = Templates::from_json(FREE).unwrap();
    let wild = templates.pool_id("wild").unwrap();
    for (seed, size) in [(1u64, 1u32), (2, 3), (3, 9), (99, 16)] {
        assert_eq!(
            templates.roll_matched(seed, size, size, wild, &[]).unwrap(),
            templates.roll(seed, size, size, wild, &[]).unwrap(),
            "seed {seed} size {size}"
        );
    }
    let small = templates.roll_matched(99, 3, 3, wild, &[]).unwrap();
    let big = templates.roll_matched(99, 9, 9, wild, &[]).unwrap();
    for y in 0..3 {
        for x in 0..3 {
            assert_eq!(small.template_at(x, y), big.template_at(x, y));
        }
    }
}

/// Decision, the gate: the search is bounded and says which way it ended. A layout that cannot
/// join is refused with `NoLayout`, proven; one the budget could not settle is `SearchLimit`.
/// Alternative rejected: giving up with a layout that has a seam, or searching without a bound.
/// Would change if: an impossible set of reservations returns a layout, or the search is unbounded.
#[test]
fn an_impossible_layout_is_refused_by_name() {
    let templates = Templates::from_json(ALTERNATING).unwrap();
    let chain = templates.pool_id("chain").unwrap();
    let upright = templates.template_id("upright").unwrap();
    let sideways = templates.template_id("sideways").unwrap();
    let at = |x, y, template| Reservation { x, y, template };
    // Two uprights side by side need east "open" against west "wall".
    assert_eq!(
        templates.roll_matched(1, 4, 1, chain, &[at(0, 0, upright), at(1, 0, upright)]),
        Err(RollError::NoLayout)
    );
    // The row alternates, so an upright first and a sideways second works, and the rest follows.
    let layout = templates
        .roll_matched(1, 4, 1, chain, &[at(0, 0, upright), at(1, 0, sideways)])
        .unwrap();
    assert_eq!(layout.template_at(2, 0), Some(upright));
    assert_eq!(layout.template_at(3, 0), Some(sideways));
}

/// Decision, the gate: the result is a pure function of the seed, the size, the pool and the
/// reservations, written down as a hash so the WebAssembly build checks the same layout.
/// Would change if: the order chunks try templates, or the stream they draw from, changes.
#[test]
fn a_matched_layout_hash_is_the_same_everywhere() {
    let templates = Templates::from_json(ALTERNATING).unwrap();
    let chain = templates.pool_id("chain").unwrap();
    let layout = templates.roll_matched(2026, 12, 12, chain, &[]).unwrap();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for y in 0..12 {
        for x in 0..12 {
            hash = (hash ^ layout.template_at(x, y).unwrap().0 as u64)
                .wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    assert_eq!(format!("{hash:016x}"), "433fc85150c62455");
}

#[test]
fn sockets_are_all_or_nothing_and_matching_edges_must_look_alike() {
    let half = FREE.replacen(
        r#",
      "sockets": { "north": "any", "east": "any", "south": "any", "west": "any" } }"#,
        " }",
        1,
    );
    assert_eq!(
        Templates::from_json(&half),
        Err(TemplatesError::Sockets("meadow".to_string()))
    );
    // "any" on a rocks edge and on a meadow edge differ in their tiles.
    let mismatch = FREE.replace(
        "\"rocks\", \"rows\": [\"..\", \"..\"]",
        "\"rocks\", \"rows\": [\"##\", \"..\"]",
    );
    assert_eq!(
        Templates::from_json(&mismatch),
        Err(TemplatesError::SocketTiles("any".to_string()))
    );
    let plain = Templates::from_json(&plain(FREE)).unwrap();
    let pool = plain.pool_id("wild").unwrap();
    assert_eq!(
        plain.roll_matched(1, 2, 2, pool, &[]),
        Err(RollError::NoSockets)
    );
}
