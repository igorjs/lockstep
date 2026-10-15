<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-calendar

The calendar: which season a game day is in, how long its daylight lasts and how warm it is, all
in integers.

## Calendar

`Calendar::from_json(text)` reads the year and checks it.

```json
{
  "days_per_year": 12,
  "daily_swing_celsius": 4,
  "seasons": [
    { "name": "summer", "sunrise_minute": 300, "sunset_minute": 1200, "mean_celsius": 24 },
    { "name": "winter", "sunrise_minute": 480, "sunset_minute": 960, "mean_celsius": 0 }
  ]
}
```

- The year is cut into equal seasons in order, each cut rounded down. The year repeats.
- Refused with a reason: bad JSON, no seasons, repeated names, fewer days than seasons, and a
  sunrise that is not before the sunset or a sunset past the end of the day.
- `season(day)` is the season of a game day.
- `daylight(day)` is the sunrise and sunset minutes. A season's values hold at the middle of the
  season and change in a straight line to the next season's middle, wrapping from the last season
  to the first (decision 0027). The longest day is in the middle of summer and the shortest in the
  middle of winter, and no day jumps at a season boundary.
- `mean_temperature(day)` is the day's average degrees, found the same way.
- `temperature(day, minute)` adds a swing: lowest at 03:00 and highest at 15:00, rising and
  falling in straight lines, by `daily_swing_celsius` either side of the mean.
- `apply_daylight(day, clock)` gives a day's sunrise and sunset to the core `Clock`
  (`Clock::set_daylight`). The host calls it before a step on each day, because a simulation sees
  only a shared clock. A simulation reads the same numbers from the calendar by the day.

## Almanac

`Almanac` remembers the last season seen. `observe(calendar, day)` gives
`CalendarEvent::SeasonChanged { season }` once, on the first day in another season. A new almanac
starts in the season of its day and reports nothing for it. It is a `Message`, so it saves.

## Tests

- `tests/behaviour/reading.rs`: seasons read, each refusal, the year repeating.
- `tests/behaviour/daylight.rs`: exact values at season middles, straight lines between them, the
  gate (winter shortens the day), the wrap into spring, and the clock's night moving.
- `tests/behaviour/temperature.rs`: season means, the daily swing, midwinter colder than
  midsummer at every hour, and minutes past a day wrapping.
- `tests/behaviour/almanac.rs`: one event for each change, a mid-year start, and a save.
- `tests/decisions/`: seasons hold at their middles (0027), and two years of days hash to the
  committed value natively and under WebAssembly.
