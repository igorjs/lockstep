<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-combat

Real-time combat for [Lockstep](https://github.com/igorjs/lockstep), resolved on the grid in
integers.

- `resolve`: one hit in a fixed order (evasion, block, critical, armour, resistance, floor, stagger,
  knockback).
- `hits`: the bodies a shape hits (adjacent, reach, line, arc, around, one cell), on any topology.
- `knock_back`: pushes a body through occupancy until a wall, the edge or another body.
- `step_combat`: actions with wind-up, active and recovery windows, buffered orders, and dodges
  with invulnerability, perfect timing and counters, for every `Fighter` in a step.
- `step_projectiles`: projectiles that fly a line, hit the first body, fly over low walls, descend
  and land, or only make noise.

```rust
use lockstep_combat::{resolve, DamageKind, DamagePacket, Defence, Tags};
use lockstep_core::math::Fixed32;
use lockstep_core::{Chance, SmoothedState, Streams};

let packet = DamagePacket {
    amount: Fixed32::from_int(100),
    kind: DamageKind::Cut,
    knockback: 1,
    stagger: Fixed32::from_int(5),
    critical: Chance::NEVER,
    critical_multiplier: Fixed32::ONE,
    tags: Tags::NONE,
};
let mut defence = Defence { armour: Fixed32::from_int(30), ..Defence::default() };
defence.resistances[DamageKind::Cut.index()] = Fixed32::HALF;
let result = resolve(&packet, &defence, &mut SmoothedState::default(), &mut Streams::new(1));
assert_eq!(result.dealt, Fixed32::from_int(35));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-combat.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
