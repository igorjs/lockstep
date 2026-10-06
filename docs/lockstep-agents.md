<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-agents

Local awareness: agents sense through cones and hearing, so a crowd converges only on what it
saw or heard. Everything is integers. Perception, noise, memory, alert states, the director and
steering are in place; utility decisions follow in the same milestone.

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
  halved when they have no line of sight to the noise from their own eye height (a wall or a rise
  muffles it; a listener whose ears clear a low wall hears in full), and within their own hearing
  range. A negative wind strength counts as still air.

See decision 0018.

## Memory, alert states and the director

Each agent has a `Mind`: its `Alertness` (Idle, Curious, Searching, Alert), a `LastKnown` memory
(a position, the target when one was seen, a confidence and an age), and how long its target has
been out of sight. An agent hunts its memory, not the target.

`think(minds, leashes, stimuli, minutes, rules, director, surroundings, events)` runs one step in
handle order. Each agent's stimuli apply first, in the order given; then, when it saw nothing this
step, its memory ages, so a noise heard this step ages with it and a sighting starts fresh:

- Each `Stimulus` outside the agent's `Leash` (a home cell and a radius) is ignored.
- `Saw` sets a fresh memory at confidence 1 and makes the agent Alert.
- `Heard` makes an Idle agent Curious, and replaces the memory only when the noise's starting
  confidence (`heard_confidence`) is at least the memory's. An Alert agent ignores noises.
- Without a sighting, the memory ages by `minutes` and its confidence fades linearly from where it
  started to zero at `forget_after_minutes`, computed from the age so it never drifts. Alert out of
  sight for `lose_sight_after_minutes` turns Searching; a forgotten memory turns the agent Idle.
- The `Director` then grants Alert on each target to at most `alert_budget` agents: nearest the
  target's freshest known position first (the youngest memory of it), then those already Alert,
  then by handle. The rest turn Curious, with one `Held` event when the hold starts; the hold lasts
  while the agent stays Curious about that target.
- Every change is a `Changed` event, after the director has ruled.

`Leash::allows` says whether a cell is within the leash, for steering too. The director's waves,
which space a horde's arrival into pulses, wait for a consumer (deferral D5 in the roadmap).

See decision 0019.

## Steering

`steer(map, occupancy, field, agents, leashes, cell_metres, events)` moves each agent one cell
down a `FlowField`, in handle order. An agent takes the field's next cell when the map still lets
it step there and no other body holds it; otherwise `lockstep_combat::sidestep_where`: the two
neighbours either side of the way, nearest the intended direction first, among the cells it may
take. Fighters use the same sidestep. Within a leash any cell inside it may be taken; an agent
outside its leash, knocked or placed there, may take any cell no farther from home than its own,
so it finds its way back. A wall raised after the field was built is stepped around. Moving through occupancy reserves the cell at once, so no later agent in the same
step takes it and no two bodies ever share a cell. An agent that cannot move gets `Waited`; one at
the field's goal, or out of its reach, stays put without an event.

Agents stop on the first goal cell they reach, so a goal spread across a room fills from its near
edge and can block the way in; put a crowd's goal where arrivals pack in from the back. Call
`steer` on the steps an agent should move, to set its speed.

See decision 0020.

## Tests

- `tests/behaviour/sight.rs`: the cone's range and angle edges, the circle behind, a wall, and
  staggered perception.
- `tests/behaviour/hearing.rs`: listeners at the edge of a noise, the source left out, the wind
  less its threshold, a wall halving the reach, a listener off the axes on `Square4` and
  `Square8`, a tall listener over a low wall, a negative wind, and the hearing range.
- `tests/behaviour/steering.rs`: a leashed agent stopping at its leash, a boxed-in agent
  waiting while one at the goal stays quiet, a door closed after the field was built, an agent
  walking back into its leash, and a sidestep that gives way to the other side inside the leash.
- `tests/behaviour/minds.rs`: a noise making an agent Curious, confidence fading, sighting again,
  Alert ignoring noise, and a stronger memory kept.
- `tests/decisions`: the exact wind ranges, the gale, psychic noise, and staggered checks for a
  thousand agents (decision 0018); memory fading to Searching at 2 minutes and Idle at 10 at two
  step rates, the director's budget of eight among twenty, and the leash (decision 0019); 200
  agents through a one-cell doorway without sharing a cell, and a blocked mover past a body in two
  steps (decision 0020).
