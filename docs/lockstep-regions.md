<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-regions

Regions: a graph of places, each anchored to a point, and a condition (a fire, a flood, a blight,
a contagion) that spreads across the graph.

## Regions

`Regions::from_json(text)` reads regions and links and checks them.

```json
{
  "regions": [
    { "name": "gate", "x": 0, "y": 0 },
    { "name": "pear", "x": 200, "y": 0, "receptive_percent": 60 }
  ],
  "links": [ ["gate", "pear"] ]
}
```

- A region has a name, an anchor in whole metres, and a receptiveness (100 unless set): the
  percent of the spread's chance that takes hold here.
- Links run both ways. A link written twice counts once.
- Refused with a reason: bad JSON, repeated names, a link naming an unknown region, a region
  linked to itself, a receptiveness above 100, and an anchor more than 16,000 metres from the origin (so any two anchors differ by less than the
  16.16 range).
- `neighbours(id)` lists the linked regions, lowest first. `hops(from, to)` counts the fewest
  links, or nothing when no chain of links joins them. `nearest(point)` is the region whose
  anchor is nearest a point (the lowest wins a tie): how an entity's position becomes a region.

## Spread

`Spread` holds the condition of every region: clear, active for some more intervals, or spent. A
spent region neither spreads nor takes hold again.

`SpreadRules` (read from JSON by serde):

| Field | Meaning |
|---|---|
| `base_permille` | Chance in a thousand, each interval, that an active region starts one clear neighbour in still air at full pace |
| `interval_minutes` | Game minutes between tries, at least one |
| `active_intervals` | Intervals a region stays active before it is spent, at least one |
| `wind_percent_per_metre` | Percent added for each metre a second of wind blowing straight from the active region toward its neighbour, and taken off for wind blowing the other way |

- `start(region, rules, events)` starts a clear region from outside.
- `advance(regions, rules, breeze, pace_percent, minutes, streams, events)` lets game time pass
  one whole interval at a time, carrying any part of an interval to the next call, so the same
  game time gives the same spread at any step rate (decision 0030). In each interval every
  region that was active when it began tries each clear neighbour once, in region and neighbour
  order, drawing from the `"spread"` stream. A region that takes hold is active from the next
  interval, so a condition crosses at most one link an interval. Then each region active at the
  start has one interval less left, and becomes spent at none.
- The chance of one try is `base_permille` times (100 plus the wind percent) times the receptive
  percent times the pace percent, divided down, held within 0 to 1,000. The wind percent is the
  wind's component along the link from anchor to anchor, times its strength, times
  `wind_percent_per_metre`. `pace_percent` (0 to 100) is the caller's: a season, a rainfall, a
  firebreak. A pace of 0 stops the spread.
- Events: `Started { region, from }` (`from` is nothing for an outside start) and `Spent`.

## Wind and weather

`Breeze` is a direction and a strength in metres a second. The weather (drift, gusts, fronts)
stays in `lockstep-agents` beside the wind it drives, not here (decision 0029). Convert with
`Breeze { direction: wind.direction, strength: wind.strength_metres_per_second }`.

## Tests

- `tests/behaviour/graph.rs`: reading, neighbours both ways, hops, nearest with ties, each
  refusal, and doubled links.
- `tests/behaviour/spread.rs`: one link an hour, active then spent and never again, sources named
  in events, a pace of nothing, carried partial intervals, and a save that continues exactly.
- `tests/behaviour/wind.rs`: downwind, upwind and across, a wind too strong to spread against,
  and receptiveness.
- `tests/decisions/`: the step rate (0030), the spread rate against the chance (the gate: the
  mean crossing time within 3 percent of the inverse of the chance), and the seed 42 hash, which
  is checked under WebAssembly too.
