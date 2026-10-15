<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-structures

Durable multi-cell structures: entities whose cells are walls on a map while they stand. It
builds on `lockstep-spatial`. The consumer is the [estate](estate.md) example, whose flood
barrier is one.

## Structures

`Structures` holds a `Structure` per entity (a `Column` keyed by the caller's handle): the cells
it blocks, its durability and the durability it was built with.

- `build(map, structure, cells, durability, events)` raises it, all or nothing. Every cell must
  be on the map, passable and listed once. The first cell that is not is named (`OffMap` or
  `Blocked`), and the map is left exactly as it was. A zero durability, no cells, or an entity
  that is a structure already is refused too. On success each cell becomes a wall (`Built`).
- `damage(map, structure, amount, events)` wears it down (`Damaged`, with the durability left).
  At zero it collapses (`Collapsed`): the structure is removed and its cells are passable again.
- `repair(structure, amount, events)` mends it up to the durability it was built with
  (`Repaired`); `dismantle` takes it apart on purpose (`Dismantled`).
- `at(cell)` names the structure holding a cell; `get` reads one.

## Dirty chunks

Raising and bringing down a structure go through `GridMap::set_passable`, so the 32 by 32
chunks that hold its cells are marked dirty, at the build and again at the collapse. A host
reads them with `GridMap::take_dirty_chunks`. The structures keep no list of their own (see
[decision 0028](decisions/0028-structures-and-procedural-generation.md)).

## Limits

A structure blocks cells outright. Partial cover, doors that open and close, and a cost of
building (materials from an inventory) are not here; a consumer composes them from
`lockstep-inventory` and `lockstep-crafting`.
