<!-- SPDX-License-Identifier: Apache-2.0 -->

# training (example)

The consumer scenario for `lockstep-progression`, and not a game. Four workshop technicians earn
points and experience at work and spend the points on a six-node web.

- Triggers: a shift earns 1 point and 1 experience; an incident 3 points and 2 experience, so
  technicians often ask for senior or chief before their experience allows it.
- The web, in `data/training.json`: basics (1 point); electrical or mechanical (2 each, each
  excluding the other; repair or safety +10); diagnostics (3, refunded in full; repair +5);
  senior (4, experience at least 50; safety times 1.2); and chief (6, experience at least 80;
  repair times 1.5), a keystone. Every modifier changes the attribute's maximum; the current value
  stays where it is.
- Unlearning returns half a node's cost, rounded down (diagnostics all of it), and takes its
  modifier away; a keystone, or a node another one needs, is refused with the reason.
- Experience passing 50 (`seasoned`) and 80 (`veteran`) are `Mark` events; a test reaches both.

The fixture hash in `fixtures/training.hash` covers 30,000 steps: now and then a random
technician works a shift or an incident, studies a random node it can take, or unlearns one. It is
checked by `lockstep-headless verify training` and by a WebAssembly test. The script asks for any
node, whether the technician can take it or not, so the session holds real exclusion and gate
refusals.

The milestone gate is `exclusions_lock_and_gates_enforce_across_the_session`: across the whole
session no technician holds both electrical and mechanical, every senior and chief was taken
with its experience gate met, the keystone never comes back, someone reaches it, and both an
exclusion and a gate refuse at least one study. Other
tests: the exclusion and the gate refusing with their reasons, unlearning returning half and
removing the modifier, experience marks, and a restored snapshot continuing exactly.

This example stands in for the reference's progression consumer, a six-node web with one
keystone in the survivor's world (departure P21 in the roadmap).
