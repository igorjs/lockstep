<!-- SPDX-License-Identifier: Apache-2.0 -->

# crater-survey (example)

The consumer scenario for `lockstep-agents`, and not a game. Six survey rovers work a 64 by 48
plateau of two-metre cells with a ridge down the middle; three are leashed to their crater (20
metres) and three roam.

- A lander touching down is an 80 metre noise. `hear` reaches every rover in range through the
  weather's wind, and each one that hears it gets a `Heard` stimulus.
- Every fifth step, staggered, each rover looks for landers with a 40 metre cone (60 degrees
  either side) and 6 metres all around; a sighting is a `Saw` stimulus.
- `think` turns those into memory and alertness; the director lets at most two rovers be Alert
  on one lander. Memories are forgotten after 30 game minutes.
- Every 15 steps each rover picks where to go: an accepted survey order first; otherwise, while
  not Idle, its memory when the utility of investigating (confidence times 100, plus half the
  battery) beats holding (40); otherwise home, when leashed and away. Rovers with the same goal
  steer down one cached flow field. A rover within one cell of what it remembers has arrived and
  parks. Each cell moved costs 1 percent of battery, and is a `Moved` event; a rover with nowhere
  to go, or an empty battery, recharges 1 percent each time the others move.
- An operator's survey request is a delegated task: battery above half counts for it, and each
  5 metres of distance costs a point. It is accepted at 0, delayed 10 minutes at -20, and
  otherwise refused, naming `LOW_BATTERY` or `TOO_FAR`. A spot the rover could never reach (a
  ridge, outside its leash, or cut off) is refused at once as `OUT_OF_REACH`; a request naming a
  lander is ignored.
- The weather drifts, gusts and swings with fronts from the `"weather"` stream.

The fixture hash in `fixtures/crater-survey.hash` covers 18,000 steps of a scripted session: a
lander every 30 seconds at a random spot, the earliest still down leaving 15 seconds later
whenever another is down (the world keeps the landing order), and a survey request every 40
seconds. The wind starts at the weather's mean strength. It is checked by `lockstep-headless verify crater-survey` and by a
WebAssembly test.

The milestone gate is `leashes_hold_and_the_budget_is_never_exceeded`: across the whole session no
leashed rover leaves its crater and no lander has more than two rovers Alert on it, and the
director is seen holding rovers back. Other tests:
a free rover driving to a landing and seeing it, a downwind rover hearing a landing that an
upwind one as far away misses, a low battery refusing a survey with its reason, a spot out of
reach refused, landers departing in landing order, a rover parking beside a lander, and a restored
snapshot continuing exactly.

This example stands in for the reference's agent consumers, a survivor's opponent perceiving it
and a rover leashed to its crater (departure P19 in the roadmap).
