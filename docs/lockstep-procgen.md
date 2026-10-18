<!-- SPDX-License-Identifier: Apache-2.0 -->

# lockstep-procgen

Procedural generation by assembly: a map made of chunk templates drawn from weighted pools. It
builds on `lockstep-spatial`. The consumer is the [estate](estate.md) example.

## Templates and pools

`Templates::from_json(text)` reads and checks the data.

```json
{
  "side": 2,
  "templates": [
    { "name": "lawn", "rows": ["..", ".."] }
  ],
  "pools": [
    { "name": "garden", "entries": [ { "template": "lawn", "weight": 5 } ] }
  ]
}
```

- Every template is `side` rows of `side` glyphs: `.` open ground, `#` a wall, `~` marsh
  (passable, and costing 3 to cross). The side is 1 to 64 (`MAXIMUM_SIDE`).
- A pool lists templates with weights. A pool with no entries, or weights that add to nothing
  or past a million (`MAXIMUM_TOTAL_WEIGHT`), is refused, as are repeated names, an unknown
  glyph and a pool naming a template that does not exist.
- `Templates` can only be built by `from_json`, so a roll never meets a pool it cannot draw from.

## Rolling a layout

`templates.roll(world_seed, width, height, pool, reserved)` returns a `Layout` of `width` by
`height` chunks.

- A chunk in `reserved` (a `Reservation` of a place and a template) holds that template and
  draws nothing. Reservations off the layout, naming an unknown template, or naming a place twice
  are refused. So is a layout of more than a million chunks (`MAXIMUM_CHUNKS`).
- Any other chunk draws one number below the pool's total weight from the `"procgen"` stream of
  its own `Streams`, seeded by `chunk_seed(world_seed, x, y)`, and takes the entry whose running
  total passes it.
- `chunk_seed` is a function of the world seed and the chunk's place alone, built from two
  rounds of the splitmix64 finalizer, each a bijection of 64 bits. For one world seed no two
  places share a seed. So a chunk rolls the same in a bigger layout, a reservation elsewhere does
  not move it, and neighbours never repeat each other's rolls.

## Applying

`layout.apply(&templates, &mut map)` writes every chunk onto a `GridMap` from its top left,
through the map's setters, so the map's dirty chunks are marked. A map smaller than the layout
is refused and left alone. A layout naming a template the templates do not hold is refused
before anything is written.

## Matching edges

A template may give each side a socket name, so neighbouring chunks join up
([decision 0051](decisions/0051-matching-template-edges.md)).

```json
{ "name": "crossing",
  "sockets": { "north": "gate", "east": "gate", "south": "gate", "west": "gate" },
  "rows": ["##....##", "#......#", "........", "........", "........", "........", "#......#", "##....##"] }
```

- Sockets are all or nothing across a set (`TemplatesError::Sockets`), and every edge with the
  same socket must have the same tiles, read left to right on the north and south sides and top
  to bottom on the east and west (`TemplatesError::SocketTiles`).
- `templates.roll_matched(world_seed, width, height, pool, reserved)` returns a layout where the
  east socket of a chunk equals the west socket of its east neighbour, and the south socket the
  north socket of the one below. The border of the layout is free. `roll` ignores sockets.
- Chunks are placed in reading order. Each tries the pool in an order drawn by weight without
  repeats from its own `chunk_seed` stream, and takes the first template that agrees with
  placed and reserved neighbours and leaves every free neighbour a template that still agrees.
  When none fits the search steps back. With nothing in the way the matched roll equals `roll`.
- The search spends at most `search_budget(chunks)` tries. `RollError::NoLayout` means no layout
  exists (reservations that cannot sit together are refused before any search);
  `RollError::SearchLimit` means the budget ran out first; `RollError::NoSockets` means the set
  has no sockets.

## Limits

Without `roll_matched`, templates are not matched on their edges. A matched layout can differ
from the plain one when a try is refused, so a chunk is then no longer independent of its
neighbours. The search is chronological backtracking with one step of lookahead, not a complete
solver: a hard set may end in `SearchLimit` although a layout exists.
