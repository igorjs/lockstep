<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-agents

Local awareness: agents sense through cones and hearing, so a crowd converges only on what it
saw or heard. Everything is integers. This first part is perception and noise; memory, alert
states, the director, steering and utility decisions follow in the same milestone.

Distances are metres, turned into cells by the caller's cell size (`cell_metres`, half a metre in
the reference). Angles are `Turn`s, counter-clockwise from east.

## Sight

`Senses` holds a sight `Cone`, a hearing range and an eye height. `sees(map, from, facing, senses,
target, cell_metres, line)` is true when the target is within the cone's range and half-angle of
the facing, or within its circle all around whatever the facing, and a line of sight joins them
at eye height.

`perceive(map, occupancy, senses, facing_of, targets, step, every, cell_metres, out)` lists every
agent seeing every target, in handle order. Checks are staggered: agent n checks only on steps
where `step % every == n % every`, n its slot (`checks_on`), so each step checks about one agent
in `every`, and each agent checks once every `every` steps.

## Noise and wind

A `Noise` has a cell, a loudness in metres (how far it carries in still air), an optional source,
and whether it is psychic. `Wind` has a direction (where it blows toward) and a strength.

- `effective_range(loudness, wind, toward_listener)` is `loudness × (1 + 0.04 × strength × cos θ)`,
  θ between the wind and the direction from the source to the listener, in exact integers rounded
  once. A 6 metre footstep in a 10 metre a second wind carries exactly 8.4 metres downwind, 3.6
  upwind and 6 across.
- `hearing_threshold(wind)` is 0.3 metres per metre a second. `audible_metres` is the effective
  range less the threshold, never below zero: a 14 metre a second gale cuts a 6 metre footstep to
  1.8 metres across the wind and to nothing against it. A psychic noise ignores the wind both ways
  and is heard to its loudness.
- `hear(map, occupancy, senses, noise, wind, cell_metres, out)` lists the bodies with `Senses`
  that hear it, sorted by handle, never the source: within the audible distance toward them,
  halved when no line of sight joins them at ear height (a wall or a rise muffles it), and within
  their own hearing range.

See decision 0018.

## Tests

- `tests/behaviour/sight.rs`: the cone's range and angle edges, the circle behind, a wall, and
  staggered perception.
- `tests/behaviour/hearing.rs`: listeners at the edge of a noise, the source left out, the wind
  less its threshold, a wall halving the reach, and the hearing range.
- `tests/decisions`: the exact wind ranges, the gale, psychic noise, and staggered checks for a
  thousand agents (decision 0018).
