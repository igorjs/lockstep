<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0029 The weather stays in lockstep-agents (decided)

Status: decided in milestone M13. It settles departure P18 and amends decision 0022.

## Decision

- The weather (drift, gusts, fronts) stays in `lockstep-agents` beside the `Wind` it drives and
  the hearing that reads it. `lockstep-regions` takes the wind as a plain `Breeze` (a direction and
  a strength) and depends on neither `lockstep-agents` nor the weather.
- Regions are one reader of the wind. The seed 42 weather hash, the `"weather"` stream and the
  whole-second advance of decision 0022 are unchanged.

## Alternatives rejected

- Moving the weather into `lockstep-regions`: agents would then depend on regions for hearing,
  and every consumer of the wind would pull in a graph it does not use. It would also touch a
  committed hash and a passing milestone for no new behaviour.
- A third crate for the weather: more surface for a type with no second consumer.

## Would change if

A second crate needs the weather without agents (a consumer of regions that wants weather but has
no agents). Then the weather moves to a crate both depend on, with a commit naming the weather
rule, and `crates/lockstep-agents/tests/decisions/weather_from_seed_42.rs` keeps its hash.
