// SPDX-License-Identifier: Apache-2.0
//! Decision: the weather stays in `lockstep-agents`, and regions take its wind as a plain
//! `Breeze`, built with a direction and a strength, so a spread is driven by real weather
//! without regions depending on agents at run time.
//! Alternative rejected: moving the weather into regions, which would make hearing in agents
//! depend on a region graph it does not use.
//! Would change if: a wind from `Weather::wind()` cannot be turned into a `Breeze` with those two
//! fields, or a spread driven by it differs between two runs of the same seed.

use crate::common::{hours, id, regions, sure};
use lockstep_agents::{Weather, WeatherEvent, WeatherRules};
use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use lockstep_regions::{Breeze, Spread, SpreadRules};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn spread_under_weather(seed: u64) -> Spread {
    let regions = regions();
    let rules = SpreadRules {
        base_permille: 300,
        wind_percent_per_metre: 5,
        ..sure()
    };
    let weather_rules = WeatherRules {
        mean_strength: Fixed32::from_int(6),
        fronts_per_day: 6,
    };
    let mut weather = Weather::new(0, Fixed32::from_int(6));
    let mut streams = Streams::new(seed);
    let mut spread = Spread::new(&regions);
    spread.start(id(&regions, "a"), &rules, &mut Vec::new());
    let mut weather_events: Vec<WeatherEvent> = Vec::new();
    for _ in 0..24 {
        weather.advance(
            Fixed32::from_int(60),
            &weather_rules,
            &mut streams,
            &mut weather_events,
        );
        let wind = weather.wind();
        let breeze = Breeze {
            direction: wind.direction,
            strength: wind.strength_metres_per_second,
        };
        hours(&mut spread, &regions, &rules, breeze, 100, 1, &mut streams);
    }
    spread
}

#[test]
fn a_spread_driven_by_the_weather_repeats_from_the_same_seed() {
    let first = spread_under_weather(42);
    assert_eq!(first, spread_under_weather(42));
    assert!(first.active_count() + first.spent_count() > 1);
}
