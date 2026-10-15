<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-regions

Places for [Lockstep](https://github.com/igorjs/lockstep): a graph of regions, each anchored to a
point, and a condition (a fire, a flood, a blight) that spreads across the graph before the wind.

- `Regions`: regions and their links, read from JSON and checked. `neighbours`, `hops` and
  `nearest` (the region a point is anchored to) answer questions about the graph.
- `Spread`: the condition of every region (clear, active or spent). `advance` lets game time pass:
  each interval, every active region may start each clear neighbour, more often downwind and in
  step with a pace the caller supplies (a season, say), drawing from the `"spread"` stream.

The weather that makes the wind stays in `lockstep-agents` (decision 0029); pass its wind to
`advance` as a `Breeze`.

## Example

```rust
use lockstep_core::math::Fixed32;
use lockstep_core::Streams;
use lockstep_regions::{Breeze, Regions, Spread, SpreadRules};

let regions = Regions::from_json(r#"{
  "regions": [ { "name": "west", "x": 0, "y": 0 }, { "name": "east", "x": 100, "y": 0 } ],
  "links": [ ["west", "east"] ]
}"#).unwrap();
let rules = SpreadRules { base_permille: 1_000, interval_minutes: 60, active_intervals: 3,
                          wind_percent_per_metre: 0 };
let mut spread = Spread::new(&regions);
let mut events = Vec::new();
spread.start(regions.region_id("west").unwrap(), &rules, &mut events);
let calm = Breeze { direction: 0, strength: Fixed32::ZERO };
spread.advance(&regions, &rules, calm, 100, Fixed32::from_int(60), &mut Streams::new(7), &mut events);
assert!(spread.is_active(regions.region_id("east").unwrap()));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-regions.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
