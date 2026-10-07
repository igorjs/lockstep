<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-progression

Progression for [Lockstep](https://github.com/igorjs/lockstep): a graph of nodes bought with points
earned by named triggers.

- `Graph`: nodes with costs, prerequisites, exclusions (both ways), gates over attributes,
  keystones that never refund, refund percents and modifiers, and the triggers that earn points,
  read from JSON and checked: no loops, no contradictions.
- `award`, `unlock` and `refund`: points from a trigger; take a node (checked in order:
  requirements, exclusions, gates, points), applying its modifiers to the owner's attributes; and
  give it back for its refund percent, unless it is a keystone or another taken node needs it.
- `why_not`: the first reason a node cannot be taken now.

## Example

```rust
use lockstep_attributes::{Attributes, Registry};
use lockstep_core::{Column, StableVector};
use lockstep_progression::{award, enrol, unlock, Graph, Owners, Refusal};

let registry = Registry::from_json(r#"{ "attributes": [] }"#).unwrap();
let graph = Graph::from_json(r#"{
  "triggers": [ { "name": "shift", "points": 1 } ],
  "nodes": [
    { "name": "electrical", "cost": 1, "excludes": ["mechanical"] },
    { "name": "mechanical", "cost": 1 }
  ]
}"#, &registry).unwrap();
let who = StableVector::new().insert(());
let (mut progress, mut attributes) = (Column::new(), Column::<Attributes>::new());
enrol(&mut progress, who);
let (mut events, mut attribute_events) = (Vec::new(), Vec::new());
award(&mut progress, who, graph.trigger_id("shift").unwrap(), &graph, &mut events).unwrap();
award(&mut progress, who, graph.trigger_id("shift").unwrap(), &graph, &mut events).unwrap();
let mut owners = Owners { attributes: &mut attributes, registry: &registry, events: &mut attribute_events };
let (electrical, mechanical) = (graph.node_id("electrical").unwrap(), graph.node_id("mechanical").unwrap());
unlock(&mut progress, who, electrical, &graph, &mut owners, &mut events).unwrap();
// The exclusion was written on electrical; it locks mechanical all the same.
assert_eq!(
    unlock(&mut progress, who, mechanical, &graph, &mut owners, &mut events),
    Err(Refusal::ExcludedBy(electrical))
);
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-progression.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
