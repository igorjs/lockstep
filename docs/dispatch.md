<!-- SPDX-License-Identifier: Apache-2.0 -->

# dispatch (example)

The consumer scenario for `lockstep-relations`, and not a game. A dispatcher on the day shift
assigns routes to four drivers, two on the day shift and two on nights.

- A driver answers an assignment as a delegated task (`lockstep_agents::evaluate_task`) with two
  considerations: `TRUST`, its trust in the dispatcher less 10, and `FATIGUE`, 5 against for each
  route it has taken today. It accepts at 0 or more, delays 30 minutes at -15 or more, and
  otherwise refuses, naming the lower-scoring reason.
- Trust is a relation in `data/relations.json`: from -100 to 100, starting at 20 and drifting
  back toward it 5 a game day, with thresholds `trusted` (50), `doubtful` (10) and `hostile`
  (-30). A driver's trust in the dispatcher is the pair standing plus how its standing toward the
  dispatcher's shift has moved (`Relations::toward`).
- Pay on time adds 4 to the driver's trust; late pay takes 12, and takes 3 from each of its shift
  mates' standing toward the dispatcher's shift.
- Routes taken reset each game day. A delay adds no route.

The fixture hash in `fixtures/dispatch.hash` covers 36,000 steps: now and then a route for a
random driver, and a third as often pay for one, late one time in five. It is checked by
`lockstep-headless verify dispatch` and by a WebAssembly test.

The milestone gate is `a_refusal_naming_trust_comes_only_below_the_threshold`: across the session
every refusal naming trust comes while that driver's trust is below 10, and there are such
refusals. Other tests: a driver paid late twice refusing and naming trust, a tired driver
delaying and then refusing for fatigue once trust falls, late pay souring the shift mates and no
one else, and a restored snapshot continuing exactly.

This example stands in for the reference's relations consumer, a companion refusing a task
below a trust threshold (departure P22 in the roadmap).
