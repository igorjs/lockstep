<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0030 A condition spreads once per interval, by chance (decided)

Status: decided and implemented in milestone M13.

## Decision

- The spread tries once per whole interval of game minutes, carrying any part of an interval to
  the next step, and draws from the `"spread"` stream only then. The same game time gives the same
  spread at any step rate.
- Each interval, every region active at its start tries each clear neighbour once, in region and
  neighbour order. A region that takes hold waits for the next interval, so a condition crosses
  one link an interval.
- A try is a draw in a thousand against the chance, so the mean time to cross a link is the
  inverse of the chance. The wind, the receptiveness and the caller's pace scale the chance; a
  pace of 0 stops it.
- A spent region never takes hold again.

## Alternatives rejected

- A chance per step: the spread would depend on the frame rate.
- A fixed delay per link: no spread would be faster by luck, wind or season.
- Spreading within the interval a region took hold: the order of regions would change how far a
  condition travels.

## Would change if

Any of these fails: `spread_ignores_the_step_rate.rs` (a day at 30 and at 60 steps a second
differs), `spread_rate_follows_the_chance.rs` (the mean crossing time over 20,000 trials is more
than 3 percent from the inverse of the chance), or `spread_from_seed_42.rs` (the hash changes
without a commit naming the spread rule that changed), all in
`crates/lockstep-regions/tests/decisions/`.
