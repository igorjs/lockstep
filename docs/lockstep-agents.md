<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-agents

Local awareness: agents sense through cones and hearing, so a crowd converges only on what it
saw or heard. Everything is integers: perception, noise, memory, alert states, the director,
steering, and utility decisions with delegation.

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

## Weather

`Weather` is the wind over time, advanced by `advance(minutes, rules, streams, events)` one whole
game second at a time from the `"weather"` stream, so the same game minutes give the same weather
at 30 or 60 steps a second. Each second:

- Outside a front, the direction drifts by at most 12 turn units (under 4 degrees a game minute).
- A front starts on average `fronts_per_day` times a day and swings the direction 90 to 180
  degrees either way over 8 to 12 game minutes, on a straight schedule, ending exactly where it
  swung to (`FrontStarted`, `FrontPassed`).
- The strength moves a thirtieth of the way to `mean_strength` each game minute (stopping within
  about three hundredths of a metre a second of it, where the step rounds to nothing).
- A gust starts on average once or twice an hour (1.5 in 3,600 seconds) and adds 50 to 100
  percent for 30 to 90 game seconds (`GustStarted`, `GustEnded`).

`wind()` is the `Wind` now, gusts included, for `hear`. The reference keeps the weather with the
regions; it lives here until that crate exists (departure P18). See decision 0022.

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

## Utility decisions and delegation

A `Consideration<W>` scores an agent against the world `W` as a plain integer a designer can read;
any `Fn(Handle, &W) -> i32` is one. A `Choice` sums its considerations in 64 bits, so totals
never wrap, and `scores` lists each one for showing why. `choose(agent, choices, world)` returns
the id of the highest total, the lowest id on a tie.

Delegation is an intent an agent answers by score. A `Task` lists considerations, each with the
`Reason` it stands for, and two thresholds. `evaluate_task(agent, task, world)` returns `Accept` at
`accept_at` or above, `Delay { minutes }` at `delay_at` or above, and otherwise
`Refuse { reason }`, naming the consideration that scored lowest (the first listed on a tie) so the
host can show it; a task with no considerations is refused with no reason. Relations and attributes feed the scores through the world the considerations
read.

See decision 0021.

## Benchmark

`cargo bench -p lockstep-agents` (or `just bench`) times 1,000 agents on a 256 by 256 map of
half-metre cells with 10 percent walls, each the best of five runs, against the committed
`benches/baseline.txt`; a measurement more than 10 percent slower prints a warning and never fails
the build. `just bench-baseline` writes a new baseline after an intended change. On the
development MacBook: every agent looking for ten targets with no staggering, 625 microseconds;
one 80 metre noise through a 10 metre a second wind, 438; one `think` with a sighting for every
agent, 146; one `steer` step for every agent, 82; a game minute of weather, 2. With checks
staggered one in ten, a step of 1,000 agents costs about 0.3 milliseconds.

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
  steps (decision 0020); utility ties going to the lowest id, and a refused task naming its
  weakest reason (decision 0021); one game day of weather from seed 42 equal at 30 and 60 steps a
  second and matching `tests/fixtures/weather.hash` natively and under WebAssembly (decision
  0022).
- `tests/behaviour/weather.rs`: a day keeping the drift, gust and front bounds, about 36 gusts,
  the strength moving toward the mean, a gust's percent, and a front ending where it swung.
- `tests/behaviour/utility.rs`: per-consideration scores, totals that never wrap, a consideration
  as a type, and a task with none.
