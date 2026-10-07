<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-progression

Progression: a graph of nodes bought with points earned by named triggers, with prerequisites,
exclusions, behavioural gates, irreversible keystones and refunds.

## The graph

`Graph::from_json(text, &attribute_registry)` reads the nodes and triggers and checks them.
Numbers in gates and modifiers are whole numbers or decimal strings, as in the attribute registry.

```json
{
  "refund_percent": 50,
  "triggers": [ { "name": "shift", "points": 1 }, { "name": "incident", "points": 3 } ],
  "nodes": [
    { "name": "basics", "cost": 1 },
    { "name": "electrical", "cost": 2, "requires": ["basics"], "excludes": ["mechanical"],
      "modifiers": [ { "attribute": "repair", "modifier": { "add": 10 } } ] },
    { "name": "mechanical", "cost": 2, "requires": ["basics"] },
    { "name": "senior", "cost": 3, "requires": ["basics"],
      "gates": [ { "attribute": "experience", "at_least": 50 } ] },
    { "name": "chief", "cost": 4, "requires": ["senior"], "keystone": true, "refund_percent": 0 }
  ]
}
```

- A node has a cost, the nodes it requires (all of them), the nodes it excludes, gates (an
  attribute at or above a value), whether it is a keystone, a refund percent (the graph's default
  unless it sets its own), and modifiers for its owner's attributes while taken.
- An exclusion written on one node holds both ways: it is added to the other node too.
- A trigger is a name and the points it earns.
- Refused with a reason: repeated names, an unknown node or attribute, requirements that loop
  (`Cycle`), a refund over 100 percent, and a node no owner could ever take (`Contradiction`): one
  requiring or excluding itself, or whose full requirements, theirs included, hold two nodes
  that exclude each other or a node it excludes. The checks are iterative, so a long chain loads.

## Progress

Each owner has a `Progress` in a `Column`: its points, the nodes taken, and the modifiers those
nodes hold. `enrol` makes an entity an owner; only enrolled owners earn and take.

- `award(progress, owner, trigger, graph, events)` adds a trigger's points (`Awarded`).
- `why_not(progress, owner, node, graph, attributes)` is the first reason the owner cannot take
  the node now, checked in this order: unknown owner or node, already taken, a missing
  requirement, a taken node that excludes it, a gate not met, then too few points.
- `unlock(progress, owner, node, graph, owners, events)` takes the node when `why_not` finds
  nothing: it spends the cost and adds the node's modifiers to the owner's attributes, when it
  has any (`Unlocked`). Gates are checked now, once; an attribute that later falls takes nothing
  away.
- `refund(progress, owner, node, graph, owners, events)` gives a taken node back for its refund
  percent of the cost, rounded down, and removes its modifiers (`Refunded`). Refused for a
  keystone, and while another taken node requires it, so a keystone holds its whole chain.

See decision 0024.

## Tests

- `tests/behaviour/graph.rs`: every field read, exclusions on both sides, each refusal including
  nodes that could never be taken, and a 20,000-node chain.
- `tests/behaviour/progress.rs`: points from triggers, requirements, refunds rounded down with
  their modifiers removed, `why_not`'s order (requirement, exclusion, gate, points), an owner not
  enrolled, and a save and load.
- `tests/decisions`: exclusions lock both ways, gates enforce when taken and not after, and
  keystones never refund (decision 0024).
