<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-combat

Real-time combat on the grid, resolved in integers. This first part has damage, hit shapes and
knockback; actions with their timing windows, dodges, projectiles and movement come next in
milestone M6.

## Damage

`resolve(&packet, &defence, &mut evasion_memory, &mut streams)` resolves one hit in a fixed order:

1. Evasion: a smoothed roll (`roll_smoothed` on the `"evasion"` stream) against `Defence::evasion`,
   with the target's own `SmoothedState`. `Tags::UNAVOIDABLE` skips it. An evaded hit deals nothing.
2. Block: when `Defence::blocking`, the `block` fraction is taken away and the knockback cancelled.
3. Critical: a plain roll on the `"combat"` stream against `DamagePacket::critical`, multiplying by
   `critical_multiplier`.
4. Flat armour is subtracted.
5. The resistance for the packet's `DamageKind` scales what is left. Resistances run from -1 to 1:
   0.5 halves the hit, and -0.5 is a weakness that takes one and a half times.
6. The result is floored at zero.
7. The hit staggers when its stagger is greater than the target's poise.
8. The knockback is passed on when the hit landed unblocked.

100 damage against 30 armour and 50 percent resistance deals 35. Everything is `Fixed32`, computed
wide enough not to wrap.

## Hit shapes

`hits(shape, map, occupancy, attacker, from, facing, out)` lists the bodies a shape hits, sorted by
cell, never the attacker. Facing is a `Turn` counter-clockwise from east, with north a quarter turn,
on a map whose y grows south; `direction` gives the angle between two cells' centres. Reach is
counted in steps (`GridMap::steps`), and angles between cell centres (`GridMap::centre`), so a shape
means the same thing on `Square8`, `Square4` and hexagons.

| Shape | Hits |
|---|---|
| `Adjacent` | the body in the one cell ahead |
| `Reach(n)` | the first body on the line up to `n` steps ahead, stopping at a wall (a spear) |
| `Line { length }` | every body on that line up to the first wall (a beam) |
| `Arc { radius, half_angle }` | every body within `radius` steps whose direction is within `half_angle` of the facing |
| `Around(radius)` | every body within `radius` steps |
| `Cell(target)` | the body on one cell (a thrown object) |

"Ahead" is the cell `n` steps away whose direction is nearest the facing, lowest cell on a tie,
chosen as if the map had no edge: an attacker on the map's edge facing outward reaches nothing. The
straight shapes stop at a wall, at a wall corner a body could not cut, and at the edge, and pass
over the attacker's own cells. A `Cell` target off the grid hits nobody. An arc of radius 1 and
half-angle 45 degrees facing north covers three cells on `Square8` and two on hexagons.

## Knockback

`knock_back(map, occupancy, who, heading, cells)` pushes a body along the line toward the cell
`cells` steps ahead, chosen as if the map had no edge, so a push into the edge stops instead of
sliding along it and a push between two neighbours alternates instead of drifting. Every cell of a
larger body takes the same step, so it keeps its shape. It stops at the first wall, edge or body,
moves through occupancy, and returns where the anchor ended, how far it moved, and the `Impact`
(`Wall` or `Body`); the simulation decides the impact damage. A body not on the map is left alone.

## Tests

- `tests/behaviour`: every step of the damage order, smoothed evasion, every shape on `Square8` and
  `Square4`, and knockback that clears, stops at a body, and stops at the edge.
- `tests/decisions`: armour before resistance, arc coverage per topology, knockback stopping at
  walls. See decision 0013.
- The hexagon cases run with the `hex` feature, natively and under WebAssembly.
