<!-- SPDX-License-Identifier: Apache-2.0 -->

# estate (example)

The consumer scenario for `lockstep-structures` and `lockstep-procgen`, and not a game. A garden
estate is laid out from chunk templates, and a flood barrier is raised across it.

- The estate is 6 by 4 chunks of 8 by 8 cells, 48 by 32 cells. The templates, in
  `data/templates.json`, are cloister and lawn (open), orchard (trees), pond (marsh around a
  wall) and hedge (long walls with gaps). The `garden` pool weighs lawn 5, orchard 3, pond 2 and
  hedge 2.
- Four chunks on the middle row are reserved as cloister: both ends and the two in the middle.
  Every other chunk is drawn from the pool, each from its own seed, so the same simulation seed
  always rolls the same estate.
- Orders: raise a barrier along a row from a column and row (refused if any cell is a wall
  already, is off the map or repeats); a flood surge of some force wears every barrier down
  (4 durability per cell it covers); mend every barrier; survey whether a walker can still get
  from the west cloister to the east one.
- Each step reports the chunks whose cells changed (`Redrawn`) after the orders run.

The fixture hash in `fixtures/estate.hash` covers 3,000 steps of barriers raised in random
places, some refused, surges, mending and surveys. It is checked by `lockstep-headless verify
estate` and by a WebAssembly test.

The milestone gate is two tests. `a_barrier_blocks_and_a_collapse_opens_and_both_report_the_chunks`:
a barrier across the border of two chunks blocks its cells and reports both chunks, and when it
collapses the cells open and both are reported again. `the_layout_keeps_its_cloisters_and_draws_the_rest_from_the_pool`
together with `chunk_seeds_never_collide` (200 world seeds times 128 by 128 places, all
different) covers the generation. Other tests: refused barriers, mending, a closed estate that
opens when its barrier gives way, and the session reaching every branch.

## Joined paths and fences

`data/joined.json` is a second template set with sockets: a crossing, two lanes, a dead end and
a plot, where a gate (a gap in the wall) only meets a gate and a fence only meets a fence. The
test `paths_and_fences_join_across_every_chunk_border` rolls 8 by 6 chunks with `roll_matched`
for several seeds, checks that every seam is open on both sides or walled on both, and checks
with the pathfinder that a walker reaches exactly the chunks of its own path network and none
of another. Other tests cover reservations, a refused layout (`NoLayout`) and a search that
ends in `SearchLimit`. The estate's own fixture hash does not change.

This example stands in for the reference's barrier and generated-layout consumers
(departure P24 in the roadmap).
