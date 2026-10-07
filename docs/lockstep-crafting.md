<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-crafting

Crafting: recipes that consume their inputs atomically, take game time at a station, and end in
an outcome drawn from a table, with failure branches. It builds on `lockstep-inventory`.

## Recipes

`Recipes::from_json(text, &inventory_catalogue)` reads station kinds and recipes and checks them.

```json
{
  "stations": ["mixer", "oven"],
  "recipes": [
    { "name": "bread", "station": "oven", "minutes": 30,
      "inputs": [ { "kind": "dough", "count": 1 } ],
      "outcomes": [
        { "name": "good", "weight": 85, "outputs": [ { "kind": "bread", "count": 2 } ] },
        { "name": "burnt", "weight": 10, "outputs": [ { "kind": "charcoal", "count": 1 } ] },
        { "name": "collapsed", "weight": 5 } ] }
  ]
}
```

- A recipe names its station kind, its inputs (kinds and counts), its whole game minutes, and an
  outcome table: rows with a name, a weight and outputs. A row yielding less, or nothing, is a
  failure branch.
- Refused with a reason: repeated names, an unknown station or kind, an input or output of no
  units, an output larger than one item of its kind holds, a table with no rows or weights that
  add to nothing or past a million (`MAXIMUM_TOTAL_WEIGHT`, which keeps the draw's skew under
  three hundredths of a percent), and a recipe longer than 32,767 minutes.
- `Recipes` can only be built by `from_json`, so a job never meets a recipe it cannot finish.

## Stations and jobs

`Crafting` holds the stations (an entity with a station kind, `add_station`, and a container in
the inventory for its outputs) and the one job running at each.

- `start(inventory, recipes, station, recipe, source, events)` checks the station kind and that
  it is idle, then counts every input in the source container, across as many stacks as hold the
  kind and leaving spoiled units out. When any is short, nothing is consumed and the first short
  input is named (`Missing`). Otherwise the inputs are consumed from the container's items in
  order and the job starts (`Started`).
- `tick(inventory, catalogue, recipes, minutes, streams, events)` lets game time pass; each job
  that runs out ends, in station handle order. Its outcome is drawn from the `"crafting"` stream
  (`Finished`, with the row), and each output goes into the station's container (`Produced`), or
  is left loose when there is no room (`Overflowed`).
- `roll(weights, streams)` draws one number below the total weight and takes the row whose
  running total passes it, so each row's share converges on its weight and a weight of zero is
  never drawn.

See decision 0026.

## Tests

- `tests/behaviour/recipes.rs`: recipes read, and each refusal.
- `tests/behaviour/jobs.rs`: a job taking its minutes and putting its outcome in the station,
  refusals for the wrong recipe, a busy station and a stranger, an output left loose, the same
  seed baking the same, and a job saved halfway finishing the same.
- `tests/decisions`: consumption is atomic (a later short input leaves an earlier one alone),
  works across stacks, adds up a kind listed twice and leaves spoiled units out, and
  100,000 draws converge within half a percent of each weight (decision 0026).
