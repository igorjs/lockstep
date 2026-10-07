<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0025 Standings are directional and decay exactly (decided)

Status: decided and implemented in milestone M11.

## Decision

- A standing is how one entity stands toward another entity or a group, in that direction only.
- A standing drifts toward its relation's rest by a decay a game day, computed afresh from its
  value at the last change and the exact game minutes since, so the same game time decays it the
  same at any step rate. A change restarts the drift.
- Thresholds fire once per crossing, in passing order; reaching a threshold from below counts as
  crossing it upward, and falling below it as crossing it downward.
- How an entity stands toward another all told is the pair standing plus its standings toward
  the groups the other belongs to, held within the bounds.

## Alternatives rejected

- One shared value per pair: it cannot say that a driver distrusts a dispatcher who trusts them.
- Taking a little off each step: it rounds once per step and drifts with the frame rate.
- Group standings replacing the pair standing: a member would lose what they earned on their own.

## Would change if

`crates/lockstep-relations/tests/decisions/` fails: trust raised to 55 decays to anything but
exactly 45 in a game day, at 30 or 60 steps a second, or falls below trusted on a different
minute; or raising one entity's trust in another changes the other's trust back.
