// SPDX-License-Identifier: Apache-2.0
#![allow(dead_code)]

use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use lockstep_regions::{Breeze, RegionId, Regions, Spread, SpreadEvent, SpreadRules};

/// A row of four regions a hundred metres apart along the east, a branch north of the second,
/// and an island joined to nothing.
pub const REGIONS: &str = r#"{
  "regions": [
    { "name": "a", "x": 0, "y": 0 },
    { "name": "b", "x": 100, "y": 0 },
    { "name": "c", "x": 200, "y": 0 },
    { "name": "d", "x": 300, "y": 0 },
    { "name": "branch", "x": 100, "y": 100, "receptive_percent": 50 },
    { "name": "island", "x": 1000, "y": 1000 }
  ],
  "links": [ ["a", "b"], ["b", "c"], ["c", "d"], ["b", "branch"] ]
}"#;

pub fn regions() -> Regions {
    Regions::from_json(REGIONS).expect("the test regions are valid")
}

pub fn id(regions: &Regions, name: &str) -> RegionId {
    regions.region_id(name).expect("a known region")
}

/// Every try succeeds in still air, each hour, and a region stays active for three hours.
pub fn sure() -> SpreadRules {
    SpreadRules {
        base_permille: 1_000,
        interval_minutes: 60,
        active_intervals: 3,
        wind_percent_per_metre: 0,
    }
}

pub fn calm() -> Breeze {
    Breeze {
        direction: 0,
        strength: Fixed32::ZERO,
    }
}

/// Lets `hours` whole hours pass at the given pace.
pub fn hours(
    spread: &mut Spread,
    regions: &Regions,
    rules: &SpreadRules,
    breeze: Breeze,
    pace_percent: u32,
    hours: i32,
    streams: &mut Streams,
) -> Vec<SpreadEvent> {
    let mut events = Vec::new();
    spread.advance(
        regions,
        rules,
        breeze,
        pace_percent,
        Fixed32::from_int(60 * hours),
        streams,
        &mut events,
    );
    events
}
