<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0028 Structures change the map, and every chunk has a seed of its own (decided)

Status: decided and implemented in milestone M14.

## Decision

- A structure is built all or nothing, and raising or bringing it down changes the map only
  through the map's own setters, so the map's dirty chunks are the one record of what changed.
- A chunk of a generated map draws from a stream seeded by the world seed and the chunk's place
  alone (`chunk_seed`, two rounds of a 64-bit bijection). For one world seed no two places share
  a seed.
- Reserved chunks draw nothing and shift nothing.

## Alternatives rejected

- Walling cells one by one and stopping at a blocked one: half a structure with no owner.
- A list of changed chunks kept by the structures: it could disagree with the map's.
- One stream walked across the chunks in order: a bigger layout, or a reservation that skips a
  draw, would move every chunk after it.
- Adding the coordinates to the world seed: chunk (1, 0) of seed 0 would repeat chunk (0, 1) of
  seed 1.

## Would change if

`crates/lockstep-structures/tests/decisions.rs` or `crates/lockstep-procgen/tests/decisions.rs`
fails: a refused build changes the map, a collapse across a chunk border misses a chunk, a chunk
differs between a small and a large layout of one seed, or two of 200 world seeds times 128 by
128 places share a chunk seed.
