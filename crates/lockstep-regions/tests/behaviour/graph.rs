// SPDX-License-Identifier: Apache-2.0
use crate::common::{id, regions, REGIONS};
use lockstep_core::math::{Fixed32, Vector2};
use lockstep_regions::{RegionId, Regions, RegionsError};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn point(x: i32, y: i32) -> Vector2 {
    Vector2::new(Fixed32::from_int(x), Fixed32::from_int(y))
}

#[test]
fn regions_are_read_with_links_running_both_ways() {
    let regions = regions();
    assert_eq!(regions.len(), 6);
    let (a, b, c, branch) = (
        id(&regions, "a"),
        id(&regions, "b"),
        id(&regions, "c"),
        id(&regions, "branch"),
    );
    assert_eq!(regions.neighbours(a), &[b]);
    assert_eq!(regions.neighbours(b), &[a, c, branch]);
    assert_eq!(regions.region(branch).unwrap().receptive_percent, 50);
    assert_eq!(regions.region(a).unwrap().receptive_percent, 100);
    assert!(regions.neighbours(id(&regions, "island")).is_empty());
    assert!(regions.neighbours(RegionId(99)).is_empty());
}

#[test]
fn hops_count_the_fewest_links_and_an_island_is_unreachable() {
    let regions = regions();
    let (a, d, branch, island) = (
        id(&regions, "a"),
        id(&regions, "d"),
        id(&regions, "branch"),
        id(&regions, "island"),
    );
    assert_eq!(regions.hops(a, a), Some(0));
    assert_eq!(regions.hops(a, d), Some(3));
    assert_eq!(regions.hops(d, branch), Some(3));
    assert_eq!(regions.hops(a, island), None);
    assert_eq!(regions.hops(a, RegionId(99)), None);
}

#[test]
fn a_point_is_anchored_to_the_nearest_region_and_ties_go_to_the_lowest() {
    let regions = regions();
    assert_eq!(regions.nearest(point(20, 5)), Some(id(&regions, "a")));
    assert_eq!(regions.nearest(point(190, 40)), Some(id(&regions, "c")));
    assert_eq!(
        regions.nearest(point(900, 900)),
        Some(id(&regions, "island"))
    );
    // Halfway between a and b.
    assert_eq!(regions.nearest(point(50, 0)), Some(id(&regions, "a")));
}

#[test]
fn each_refusal_names_its_reason() {
    let with = |from: &str, to: &str| Regions::from_json(&REGIONS.replace(from, to));
    assert!(matches!(
        Regions::from_json("{"),
        Err(RegionsError::Json(_))
    ));
    assert_eq!(
        with("\"name\": \"b\"", "\"name\": \"a\""),
        Err(RegionsError::DuplicateName("a".into()))
    );
    assert_eq!(
        with("[\"c\", \"d\"]", "[\"c\", \"z\"]"),
        Err(RegionsError::UnknownRegion("z".into()))
    );
    assert_eq!(
        with("[\"c\", \"d\"]", "[\"c\", \"c\"]"),
        Err(RegionsError::SelfLink("c".into()))
    );
    assert_eq!(
        with("\"receptive_percent\": 50", "\"receptive_percent\": 101"),
        Err(RegionsError::Receptive("branch".into()))
    );
    assert_eq!(
        with("\"x\": 1000", "\"x\": 16001"),
        Err(RegionsError::Anchor("island".into()))
    );
}

#[test]
fn a_link_written_twice_counts_once() {
    let twice = REGIONS.replace("[\"a\", \"b\"]", "[\"a\", \"b\"], [\"b\", \"a\"]");
    let regions = Regions::from_json(&twice).unwrap();
    assert_eq!(regions.neighbours(id(&regions, "a")).len(), 1);
}
