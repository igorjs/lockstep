<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-calendar

Seasons for [Lockstep](https://github.com/igorjs/lockstep): which season a simulated day falls in,
how long its daylight lasts, and how warm it is at a minute of the day.

- `Calendar`: seasons read from JSON and checked. `daylight(day)` gives the sunrise and sunset
  minutes, `temperature(day, minute)` the degrees, `season(day)` the season, all in integers.
  `apply_daylight` hands a day's daylight to the core `Clock`.
- `Almanac`: remembers the last season seen and emits `SeasonChanged` when a new day enters
  another one.

## Example

```rust
use lockstep_calendar::Calendar;

let calendar = Calendar::from_json(r#"{
  "days_per_year": 8, "daily_swing_celsius": 4,
  "seasons": [
    { "name": "summer", "sunrise_minute": 300, "sunset_minute": 1200, "mean_celsius": 24 },
    { "name": "winter", "sunrise_minute": 480, "sunset_minute": 960, "mean_celsius": 0 }
  ]
}"#).unwrap();
// Day 1 is near midsummer and day 5 near midwinter: winter shortens the day.
let (summer_rise, summer_set) = calendar.daylight(1);
let (winter_rise, winter_set) = calendar.daylight(5);
assert!(winter_set - winter_rise < summer_set - summer_rise);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-calendar.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
