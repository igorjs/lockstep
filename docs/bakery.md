<!-- SPDX-License-Identifier: Apache-2.0 -->

# bakery (example)

The consumer scenario for `lockstep-crafting`, and not a game. A pantry, a mixer and an oven,
each with a container.

- The kinds, in `data/catalogue.json`: flour (spoils in 3 game days), water, yeast (1 day),
  dough (4 hours), bread (1 day) and charcoal. The bakery is at 22 degrees.
- The recipes, in `data/recipes.json`: the mixer makes dough from 2 flour, a water and a yeast in
  10 minutes (risen 95, flat 5: nothing); the oven bakes a dough in 30 minutes (good 85: two
  loaves; burnt 10: a charcoal; collapsed 5: nothing).
- Orders: deliver flour, water or yeast to the pantry (turned away when it has no room); mix from
  the pantry; bake from the mixer's dough; sell a loaf off the oven's rack.
- Each step runs the orders, then spoils everything by the step's game minutes, then lets the
  jobs run, drawing outcomes from the `"crafting"` stream.

The fixture hash in `fixtures/bakery.hash` covers 54,000 steps (about 30 game hours) of
deliveries now and then, the mixer and oven asked to work whether or not they can, and loaves
sold. It is checked by `lockstep-headless verify bakery` and by a WebAssembly test.

The milestone gate is two tests. `the_pantry_changes_only_by_deliveries_and_whole_recipes`: at
every step of the session the pantry's flour, water and yeast change by exactly what was
delivered, less one dough recipe's inputs for each mix that started, so a refused mix takes
nothing. `two_thousand_bakes_land_near_the_outcome_table`: 2,000 bakes with the committed recipe
land within 3 percent of 85, 10 and 5. Other tests: bread from dough from the pantry, selling
with nothing to sell, a delivery too big turned away, and a restored snapshot continuing exactly.

This example stands in for the reference's crafting consumer, an altar reforge with a gamble
(departure P23 in the roadmap).
