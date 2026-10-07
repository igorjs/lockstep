<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-relations

Standings: how one entity stands toward another entity or a group, in kinds such as trust,
reputation and faction standing, with thresholds and decay.

## Kinds

`Kinds::from_json(text)` reads relations and groups and checks them. Numbers are whole numbers or
decimal strings.

```json
{
  "relations": [
    { "name": "trust", "minimum": -100, "maximum": 100, "starting": 0, "decay_per_day": 10,
      "thresholds": [ { "at": 50, "name": "trusted" }, { "at": -20, "name": "wary" } ] },
    { "name": "reputation", "minimum": 0, "maximum": 100, "starting": 50, "rest": 40,
      "decay_per_day": "2.5" }
  ],
  "groups": ["night_shift", "day_shift"]
}
```

- A relation has bounds, a starting value, a rest (the starting value unless set) it drifts back
  to, a decay in points a game day (none unless set), and named thresholds, kept sorted.
- Refused with a reason: repeated names, a minimum above the maximum, a starting value or rest
  outside the bounds, a negative decay, and a threshold outside the bounds.

## Standings

`Relations` holds every standing, directional: how `from` stands toward a `Target`, either
another entity or a group. How the other stands back is a standing of its own.

- `get(kinds, relation, from, to)` is the standing, or the starting value until something
  changes it.
- `change(kinds, relation, from, to, delta, events)` moves it, held within the bounds, and emits
  `Crossed` for each threshold passed, in passing order: downward when it falls below a
  threshold, upward when it reaches one.
- `tick(kinds, minutes, events)` lets game time pass. Each standing drifts toward its rest by the
  relation's decay a day, computed afresh from its value at the last change and the exact game
  minutes since, so the same game time decays it the same at any step rate and it never passes
  its rest. A change restarts the drift from the new value.
- `toward(kinds, relation, from, to, groups_of_to)` is how `from` stands toward an entity all
  told: the pair standing plus the standing toward each group it belongs to, held within the
  bounds. This is what an agent's considerations read.
- `forget(who)` drops every standing from or toward an entity that has left.

See decision 0025.

## Tests

- `tests/behaviour/kinds.rs`: relations, groups and sorted thresholds read, and each refusal.
- `tests/behaviour/standings.rs`: bounds and crossings in passing order both ways, groups added
  by `toward`, decay stopping at rest from either side and restarting after a change, and
  `forget` with a save and load.
- `tests/decisions`: a day of decay identical at 30 and 60 steps a second, and standings that run
  one way only (decision 0025).
