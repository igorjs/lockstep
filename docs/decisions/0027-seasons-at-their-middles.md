<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0027 Seasons hold at their middles (decided)

Status: decided and implemented in milestone M13.

## Decision

- A season's values (sunrise, sunset and mean temperature) hold at the middle of the season and
  change in a straight line to the next season's middle, wrapping from the last to the first. The
  longest day is in the middle of summer and the shortest in the middle of winter.
- All of it is integer arithmetic on 65,536ths of a season, and daylight is rounded to a whole
  minute, so it is the same on every platform.
- The daily temperature swing is a triangle: lowest at 03:00, highest at 15:00.
- The calendar gives daylight to the core `Clock` through `Clock::set_daylight`, called by the
  host before each step, because a simulation only sees a shared clock. A simulation reads the
  same numbers from the calendar by day.

## Alternatives rejected

- Holding a season's values for all of its days: daylight would jump by up to a season's
  difference on the first day of the next one.
- A sine curve: transcendental floats are banned, and the table-based sine would add a second
  source of rounding for no gain at one value a day.

## Would change if

`crates/lockstep-calendar/tests/decisions/seasons_hold_at_their_middles.rs` fails (the longest day
is not midsummer, the shortest is not midwinter, or a day jumps by more than 100 minutes), or
`a_year_of_days_from_the_committed_hash.rs` changes without a commit naming the calendar rule that
changed.
