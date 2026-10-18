<!-- SPDX-License-Identifier: Apache-2.0 -->

# checkout (example)

The consumer scenario for the waves in `lockstep-agents`, and not a game. A store announces a
surge of shoppers; the surge is spaced into pulses so the checkout queue stays under a budget.

- The pulses are `data/surge.json`: 40 shoppers every 5 game minutes, the first after 2.
- The queue is `data/queue.json`: a budget of 30, two lanes open at the start, one shopper served
  per lane each game minute. The budget is a `Director`; its `headroom` is what the wave may admit.
- A pulse joins the queue as far as there is room. The rest wait outside and join as lanes serve.
- An `Announce` order starts a surge of the given size; one while a surge is still arriving, or of
  no shoppers, is `Ignored`. An `OpenLane` order opens one more lane, up to eight.
- A day is 2 real minutes, so a step is 0.4 game minutes; the pulses follow game minutes, not steps.

The fixture hash in `fixtures/checkout.hash` covers 3,600 steps: a surge on the first step, one
announced too soon, a lane opened a third of the way through, and a second surge. It is checked by
`lockstep-headless verify checkout` and by a WebAssembly test.

The gate is `the_queue_never_exceeds_the_budget_across_the_session`: after every step of the
session the queue is at most the budget, and it reaches the budget, so a pulse admitted whole would
fail it. Other tests: every announced shopper is served in the end, pulses of 40, 40 and 20 with a
hold reported, a second surge ignored, lanes stopping at eight, and a snapshot taken mid-wave
continuing exactly. See decisions 0055 and 0056.
