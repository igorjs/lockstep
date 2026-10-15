<!-- SPDX-License-Identifier: Apache-2.0 -->

# orchard (example)

The consumer scenario for `lockstep-regions` and `lockstep-calendar`, and not a game. A blight
spreads across six orchard blocks as winter shortens the days.

- The blocks are a graph in `data/blocks.json`: two rows of three, linked along rows and columns,
  with the pear and quince blocks 60 percent receptive.
- The blight is `data/blight.json`: 400 in a thousand each hour, active for 8 hours, 4 percent for
  each metre a second of wind along a link. The wind comes from the weather in `lockstep-agents`
  (drift, gusts and fronts), through a `Breeze`.
- The year is `data/calendar.json`: twelve days in four seasons, with 15 hours of daylight at
  midsummer and 8 at midwinter, and a mean of 24 degrees in summer and 0 in winter.
- The pace is the temperature: idle below 5 degrees, full from 20, straight between. So the blight
  runs through summer, slows in spring and autumn, and stalls in winter.
- A day is 2 real minutes, 3,600 steps. Before each step the host gives the clock the day's
  daylight (`Calendar::apply_daylight`).
- An `Infect` order starts a block from outside; one that is unknown, infected or spent is
  `Ignored`. A new season is reported once as `Season`.

The fixture hash in `fixtures/orchard.hash` covers 43,200 steps, a whole year: the gate on the
first step, and now and then a random block from outside. It is checked by
`lockstep-headless verify orchard` and by a WebAssembly test.

The milestone gate is three tests. `in_summer_a_zone_spreads_across_the_orchard`: from the gate in
early summer the blight reaches all six blocks in three days, five of them from a neighbour.
`winter_shortens_the_days_on_the_clock` and `in_midwinter_the_blight_does_not_spread`: at 07:00
the sun is up in midsummer and not in midwinter, and a midwinter day leaves the gate alone.
Other tests: the pace following the temperature, one event for each season change in a year, an
order for a block twice, and a restored snapshot continuing exactly.

This example stands in for the reference's regions consumer, a zone that spreads while winter
shortens the days (departure P25 in the roadmap).
