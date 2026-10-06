<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-agents

Local awareness for [Lockstep](https://github.com/igorjs/lockstep): agents see through cones and
hear through the air, all in integers. Nothing knows where anything is unless it saw or heard it.

- `Senses`: a sight `Cone` (range, half-angle, and a circle all around) gated by line of sight,
  and a hearing range.
- `perceive`: staggered checks, agent n on steps where `step % k == n % k`, so a thousand agents
  cost a tenth of a full check each step at k = 10.
- `Noise` and `Wind`: a noise carries `loudness × (1 + 0.04 × strength × cos θ)` metres, exactly,
  and the wind raises every listener's threshold by 0.3 metres per metre a second. `hear` lists
  the listeners in range, muffled by walls. Psychic noise ignores the wind.
- `think`: each agent's `Mind` hunts a memory that fades in exact game minutes, moving between
  Idle, Curious, Alert and Searching; a `Director` lets at most its budget be Alert on one target,
  and a `Leash` keeps an agent from reacting to anything far from home.
- `steer`: agents move down a flow field through occupancy, sharing `sidestep` with fighters.
- `choose` and `evaluate_task`: integer utility, ties to the lowest id, and delegation that
  accepts, delays or refuses with the reason.

## Example

```rust
use lockstep_agents::{effective_range, Wind};
use lockstep_core::math::Fixed32;

let wind = Wind { direction: 0, strength_metres_per_second: Fixed32::from_int(10) };
let footstep = Fixed32::from_int(6);
// Downwind (east), upwind (west) and across.
assert_eq!(effective_range(footstep, wind, 0), Fixed32::from_ratio(84, 10));
assert_eq!(effective_range(footstep, wind, 32_768), Fixed32::from_ratio(36, 10));
assert_eq!(effective_range(footstep, wind, 16_384), Fixed32::from_int(6));
```

See the [full documentation](https://github.com/igorjs/lockstep/blob/main/docs/lockstep-agents.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/igorjs/lockstep/blob/main/LICENSE).
