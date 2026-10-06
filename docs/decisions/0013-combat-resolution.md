<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0013 Combat resolves in a fixed order, on shapes measured in steps and angles (decided)

Status: decided and implemented in milestone M6.

## Decision

- One hit resolves in a fixed order: evasion, block, critical, flat armour, percentage resistance,
  floor at zero, stagger, knockback. Armour before resistance means 100 damage against 30 armour
  and 50 percent resistance deals 35.
- Evasion is a smoothed roll with the target's own memory, so dodging is not streaky. `Defence`
  carries the evasion chance and the block, which the spec's resolution order needs.
- Hit shapes count reach in single steps (`Topology::steps`) and angle between cell centres
  (`Topology::centre`), so one shape works on every topology. `Reach` stops at the first body and
  `Line` pierces to the first wall; the spec names both without telling them apart.
- Knockback moves through occupancy one neighbour at a time and reports what stopped it; the
  simulation turns an impact into damage.

## Alternatives rejected

- Resistance before armour: armour grows stronger against big hits, and armour plus resistance can
  cancel any hit.
- Shapes as fixed cell offsets per topology: written again for every topology, and they drift.
- Choosing cells among the neighbours on the map: at the edge a knockback slides sideways and a
  strike facing outward hits a body beside the attacker. Shapes and knockback choose on a copy of
  the grid padded on every side (by an even number of cells, which keeps hexagon rows aligned).
- Choosing each knockback step on its own: a push between two neighbours always takes the same one
  and drifts. A knockback walks the line toward the cell it is aimed at.

## Would change if

`tests/decisions` in `crates/lockstep-combat` fail: 35 from the armour case, three cells on
`Square8` and two on hexagons for the quarter swing, and a knockback stopping at a wall with the
impact reported.
