<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0015 Projectiles fly at an altitude, and a dodge lets them pass (decided)

Status: decided and implemented in milestone M6.

## Decision

- A projectile flies at an absolute altitude: its launch cell's elevation plus its height. A cell
  whose elevation plus wall height reaches the altitude stops it, the same measure line of sight
  uses, so a wall exactly its height stops it, a lower wall does not, and rising ground does too.
  On a diagonal, either cell it passes between stops it the same way. A full wall reaches any
  altitude.
- Descent lowers the altitude a level every few cells, and the projectile lands where its
  altitude meets the ground.
- A projectile hits through the same rules as a strike. A dodge moves out of its way: it flies on.
  A body wider than one cell is met once.
- Projectiles are not in occupancy and are not indexed by their own handle in the timeline.

The reference says flight checks `can_step`; a projectile is not a body, so it uses heights
instead of the climb and corner rules for walking.

## Alternatives rejected

- Comparing the projectile's height with a wall's height alone: arrows fly through hills.
- `can_step` for flight: a projectile could not leave a ledge or cross a low wall it flies above.
- Removing a dodged projectile: a dodge becomes a shield for whoever stands behind.

## Would change if

`crates/lockstep-combat/tests/decisions/projectiles.rs` fails: height 2 must stop at a wall of 2
and height 3 pass it, and a dodged projectile must fly on.
