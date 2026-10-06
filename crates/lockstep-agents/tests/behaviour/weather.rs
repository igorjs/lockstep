// SPDX-License-Identifier: Apache-2.0
use crate::common::{metres, weather_day};
use lockstep_agents::{Weather, WeatherEvent};
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn turned(before: u16, after: u16) -> u16 {
    (after.wrapping_sub(before) as i16).unsigned_abs()
}

#[test]
fn a_day_of_weather_keeps_its_rules() {
    let (minutes, events) = weather_day(7, 30);
    assert_eq!(minutes.len(), 1_440);
    // Outside fronts, the direction turns at most 4 degrees (728 turn units) a minute.
    for pair in minutes.windows(2) {
        if pair[0].front.is_none() && pair[1].front.is_none() {
            assert!(turned(pair[0].direction, pair[1].direction) <= 728);
        }
        if let Some(gust) = pair[1].gust {
            assert!((50..=100).contains(&gust.extra_percent));
            assert!(gust.seconds_left <= 90);
        }
        if let Some(front) = pair[1].front {
            assert!((16_384..=32_768).contains(&front.swing.unsigned_abs()));
            assert!((480..=720).contains(&front.seconds));
        }
    }
    let gusts = events
        .iter()
        .filter(|event| **event == WeatherEvent::GustStarted)
        .count();
    // About 36 a day at 1.5 an hour.
    assert!((15..=70).contains(&gusts), "{gusts} gusts");
    // From 4 toward a mean of 6: over half way within an hour, and never past it.
    assert!(minutes[60].strength > metres(5));
    assert!(minutes.iter().all(|weather| weather.strength <= metres(6)));
}

#[test]
fn a_gust_raises_the_wind_by_its_percent_and_a_front_ends_where_it_swung() {
    let mut weather = Weather::new(0, metres(10));
    weather.gust = Some(lockstep_agents::Gust {
        extra_percent: 50,
        seconds_left: 30,
    });
    assert_eq!(weather.wind().strength_metres_per_second, metres(15));
    assert_eq!(weather.wind().direction, 0);
    // Second by second from seed 42 until a front passes: it ends exactly where it swung to.
    let rules = lockstep_agents::WeatherRules {
        mean_strength: metres(6),
        fronts_per_day: 3,
    };
    let mut weather = Weather::new(0, metres(4));
    let mut streams = lockstep_core::Streams::new(42);
    let one_second = Fixed32::from_raw(65_536 / 60 + 1);
    let mut events = Vec::new();
    let mut started = None;
    for _ in 0..86_400 {
        let before = weather.front;
        events.clear();
        weather.advance(one_second, &rules, &mut streams, &mut events);
        if let (None, Some(front)) = (before, weather.front) {
            started = Some(front);
        }
        if events.contains(&WeatherEvent::FrontPassed) {
            let front = started.expect("a front passes after it starts");
            assert_eq!(
                weather.direction,
                front.from.wrapping_add(front.swing as u16)
            );
            return;
        }
    }
    panic!("no front passed in a day");
}
