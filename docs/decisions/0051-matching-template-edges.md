<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0051 Template edges match by socket, searched in order and bounded (decided)

Status: decided and implemented; closes deferral D6.

## Decision

- A template may name a socket for each of its four sides in the template data. Sockets are
  all or nothing across a template set, and every edge sharing a socket name must have the
  same tiles, so two chunks that agree on a socket also join seamlessly.
- Matching is opt-in: `Templates::roll_matched`. `roll` ignores sockets, so every existing roll
  and fixture hash stays as it was.
- `roll_matched` places chunks in reading order. A chunk tries the pool's templates in an order
  drawn by weight without repeats from its own `chunk_seed` stream, and takes the first that
  agrees with the neighbours already placed or reserved and leaves each free neighbour some
  template that still agrees. When none fits it steps back to the previous free chunk. The first
  try is the plain draw, so with nothing in the way the matched roll equals the plain roll.
- The search is bounded by `search_budget` (32 tries per chunk plus 1,024). It returns
  `NoLayout` when it has shown that none exists (reservations that cannot sit together are found
  before the search) and `SearchLimit` when the budget ran out first.

## Alternatives rejected

- Matching inside `roll`: it would redraw every layout of every existing fixture.
- Wave function collapse: more machinery, and propagation order is another place to be
  nondeterministic; reading order with backtracking is simple and bounded.
- One stream walked across the layout: a refused try would move every chunk after it. Each
  chunk keeps a stream of its own, so seeds never collide.
- An unbounded search: a contradiction far from the start can take exponential time.

## Would change if

`crates/lockstep-procgen/tests/matching.rs` fails: a plain roll differs for a socketed set,
a matched roll with every pairing allowed differs from the plain roll, an impossible set of
reservations returns a layout, or the matched layout hash differs between native and
WebAssembly.
