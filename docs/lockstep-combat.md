<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-combat

Real-time combat on the grid, resolved in integers: damage, hit shapes, knockback, actions and
dodges with commitment, and projectiles. Movement comes next in milestone M6.

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

## Actions and dodges

Each fighter is a state machine in a `Column<Fighter>`: its `MovesetId`, phase, facing, stamina
(with a maximum) and `Defence`, plus a buffered order, a dodge cooldown, a counter window and its
evasion memory. A `Moveset` holds a kind of fighter's `ActionDefinition`s and `DodgeDefinition`;
`step_combat` takes every moveset, and each fighter names its own. Durations are seconds in
`Fixed32`, turned once into whole steps at the simulation's steps per second (`steps_for`: nearest,
ties up), so a 0.2 second wind-up is 6 steps at 30 a second and 12 at 60. Windows (the buffer, the
perfect dodge) compare steps against seconds exactly, as integers.

- An attack winds up (telegraphed, cancellable by a dodge or a stagger its mask allows), strikes on
  its first active step only, then recovers (locked out). It costs stamina when it starts; too
  little is `Refused { Tired }`.
- A dodge is vulnerable in startup (a stagger breaks it), then moves its distance through
  occupancy (a wall shortens the move, not the invulnerability) and is invulnerable, then recovers.
  It has a cooldown from the press.
- An order sent while busy waits up to 0.15 seconds and fires on the first free step; past that it
  expires. A newer order replaces the waiting one.
- A strike on an invulnerable target is `Dodged`, unless it grabs or cannot be avoided. A dodge
  pressed at most `perfect_window_seconds` before the strike is a `PerfectDodge`: the stamina comes
  back (once per dodge, never above the maximum) and, once the fighter is free, the next attack
  within `counter_seconds` skips its wind-up.
- A staggering strike interrupts a wind-up whose action allows it, never a recovery.

`step_combat(fighters, movesets, map, occupancy, orders, steps_per_second, streams, events)` runs
one step: orders replace the buffered ones, then each fighter in handle order advances and starts
its buffered order if free, then every attack on its first active step reads its targets (all
before any resolves, so two fighters trading blows on the same step both land), then the strikes
resolve in handle order. A phase that lasts no step is skipped. A body
with no `Fighter` is still hit, with a default defence. `CombatEvent` names who did what; `Landed`
carries each target's `DamageResult` and knockback, and the simulation applies the damage.

At 30 steps a second the wind-ups in the tests take 6 steps, actives 3 and recoveries 9. The
perfect window of 0.12 seconds means a dodge 3 steps (0.100 seconds) before the strike is perfect
and 4 steps (0.133 seconds) is not.

## Projectiles

`Projectile::launch(map, owner, from, heading, &launch)` aims along the line toward the cell
`range_cells` steps ahead, chosen as if the map had no edge. `step_projectiles(projectiles,
fighters, movesets, map, occupancy, steps_per_second, streams, events)` moves each, in handle order,
up to `cells_per_step` cells:

- A cut wall corner, or a wall at least as high as the projectile, stops it (`Stopped::Wall`); a
  full wall is always high enough, and a low wall below the projectile is flown over.
- In each cell it is `Sounded` when it has a loudness.
- A body other than its owner takes its damage through the same rules as a strike: an
  invulnerable fighter dodges it and it flies on, otherwise it lands (`ProjectileHit`) and stops.
  A projectile with no damage, such as a scream, passes through bodies.
- With `descent_every`, it drops a height every that many cells and stops on the ground
  (`Stopped::Landed`).
- Out of cells: `Stopped::Spent` at its full range, `Stopped::Wall` when the line left the map.

Projectiles are not in occupancy, so they never block a body. Keep them in a `StableVector`.

## Tests

- `tests/behaviour`: every step of the damage order, smoothed evasion, every shape on `Square8` and
  `Square4`, and knockback that clears, stops at a body, and stops at the edge.
- `tests/decisions`: armour before resistance, arc coverage per topology, knockback stopping at
  walls (decision 0013); stagger against wind-up and recovery, dodging and grabbing through a dodge,
  the perfect window, and a dodge buffered through recovery (decision 0014).
- `tests/behaviour/fighter.rs`: phase timing, whiffs, refusals, buffer expiry, the counter, dodge
  movement and cooldown, knockback from a blow, and repeatable events.
- `tests/behaviour/projectiles.rs`: hitting the first body, walls, low walls, range and the map
  edge, landing, a scream that is heard but hurts nobody, and a dodge letting a projectile pass.
- The hexagon cases run with the `hex` feature, natively and under WebAssembly.
