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
  are refused.
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
is refused and left alone.

## Limits

Templates are not matched on their edges: a wall at the border of one chunk may face open ground
in the next. A consumer that needs a connected map reserves the chunks that connect it (the
estate fixes a cloister at both ends and in the middle), or checks with a path search. Edge
matching is a deferral in the roadmap.
