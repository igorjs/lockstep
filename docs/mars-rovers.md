<!-- SPDX-License-Identifier: Apache-2.0 -->

# mars-rovers (example)

The grid-shaped consumer scenario: rovers on a plateau, on any topology.

- A rover has a heading, which is an index into its topology's neighbour list, and a program of `L`, `R`
  and `M`. Left and right turn by a quarter turn on the square topologies (one step of four, or two of
  eight) and by one sixth on hexagons. `M` moves one cell along the heading.
- The classic kata answers hold unchanged on `Square4` and `Square8`: the rover that lands at 1 2 facing
  north and runs `LMLMLMLMM` ends at 1 3 facing north, and the one that lands at 3 3 facing east and runs
  `MMRMMRMRRM` ends at 5 1 facing east. The kata's y grows northward, so positions convert at the boundary.
- The same scenarios run on all three topologies: a move off the edge, into a rock, head-on between two
  rovers, a cell two rovers want, a rover following into a cell another just left, and a wrecked landing.
- Rules, in handle order: a move into an edge, rock or occupied cell does nothing; the lower handle wins a
  contested cell; a rover can move into a cell a lower handle vacated earlier in the same step, but not into
  one a higher handle will vacate later; a rover that lands on an edge, rock or occupied cell is wrecked,
  holds no cell and never moves.
- `X` rams: it moves like `M`, except a rover in the way is first pushed one cell the way the rammer faces,
  through `knock_back` from `lockstep-combat`. A rock, the edge or a third rover beyond it stops the push,
  and then nobody moves; on `Square8` so does a rock on either side of a diagonal push, which a body
  cannot cut past (a diagonal `M` only checks the cell it enters). Rams do not chain. The fixture never rams, so its hash did not change.
- A program added in a step starts running in the next one.
- The compass names: north is the first heading of every topology (north-east on hexagons), east points
  along +x, and south and west are their opposites. A rock outside the plateau is ignored.

The fixture hash in `fixtures/mars-rovers.hash` covers every scenario on every topology, and is checked by
`lockstep-headless verify mars-rovers` and by a WebAssembly test.
