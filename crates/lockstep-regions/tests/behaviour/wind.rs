// SPDX-License-Identifier: Apache-2.0
use crate::common::{calm, hours, id, regions, sure};
use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use lockstep_regions::{Breeze, Regions, Spread, SpreadRules};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const EAST: u16 = 0;
const WEST: u16 = 32_768;
const NORTH: u16 = 16_384;

fn gale(direction: u16) -> Breeze {
    Breeze {
        direction,
        strength: Fixed32::from_int(10),
    }
}

/// How often one hour of spread from `start` reaches `target`, in 2,000 tries.
fn reached(regions: &Regions, start: &str, target: &str, breeze: Breeze) -> u32 {
    let rules = SpreadRules {
        base_permille: 300,
        wind_percent_per_metre: 5,
        ..sure()
    };
    let mut count = 0;
    for seed in 0..2_000 {
        let mut spread = Spread::new(regions);
        spread.start(id(regions, start), &rules, &mut Vec::new());
        hours(
            &mut spread,
            regions,
            &rules,
            breeze,
            100,
            1,
            &mut Streams::new(seed),
        );
        count += spread.is_active(id(regions, target)) as u32;
    }
    count
}

#[test]
fn the_wind_helps_the_spread_downwind_and_hinders_it_upwind() {
    let regions = regions();
    // 10 metres a second at 5 percent a metre: 150 percent downwind, 50 percent upwind, and
    // across the wind no change. Base 300 in a thousand.
    let (downwind, calm_count, upwind, across) = (
        reached(&regions, "a", "b", gale(EAST)),
        reached(&regions, "a", "b", calm()),
        reached(&regions, "a", "b", gale(WEST)),
        reached(&regions, "a", "b", gale(NORTH)),
    );
    assert!(
        upwind < calm_count && calm_count < downwind,
        "{upwind} {calm_count} {downwind}"
    );
    assert!(downwind.abs_diff(900) < 90, "{downwind}");
    assert!(upwind.abs_diff(300) < 60, "{upwind}");
    assert!(calm_count.abs_diff(600) < 70, "{calm_count}");
    assert!(across.abs_diff(calm_count) < 80, "{across} {calm_count}");
}

#[test]
fn a_wind_stronger_than_the_rate_stops_the_spread_upwind() {
    let regions = regions();
    let strong = Breeze {
        direction: WEST,
        strength: Fixed32::from_int(40),
    };
    assert_eq!(reached(&regions, "a", "b", strong), 0);
}

#[test]
fn a_half_receptive_region_takes_hold_half_as_often() {
    let regions = regions();
    // b to c (full) against b to branch (half), in still air.
    let (full, half) = (
        reached(&regions, "b", "c", calm()),
        reached(&regions, "b", "branch", calm()),
    );
    assert!(half.abs_diff(full / 2) < 70, "{half} {full}");
}

#[test]
fn the_wind_direction_holds_between_anchors_far_apart() {
    let far = Regions::from_json(
        r#"{"regions":[{"name":"west","x":-16000,"y":0},{"name":"east","x":16000,"y":0}],
            "links":[["west","east"]]}"#,
    )
    .unwrap();
    let (downwind, upwind) = (
        reached(&far, "west", "east", gale(EAST)),
        reached(&far, "west", "east", gale(WEST)),
    );
    assert!(downwind > upwind, "{downwind} {upwind}");
}
