<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0008 Attributes as data (decided)

Status: decided and implemented in milestone M4.

## Decision

Every stat is one machine configured by data: a registry of definitions read from JSON, modifiers
on the maximum, a policy for what a new maximum does to the current value, thresholds, and derived
values from curves.

- Modifiers apply Add, then Multiply, then the last Override, whatever order they were added in.
- Each attribute picks `clamp`, `scale_current` or `ratchet` for a changed maximum.
- A threshold fires once per crossing, in the order the value passes them. The spec said
  definition order, which reports a climb through thresholds declared highest first backwards.
- Derived attributes recompute in registry order after every change.
- A ratchet turns each threshold the value rises through into its minimum. The spec only said "the
  minimum only rises"; tying the rise to thresholds gives corruption named points of no return.

## Alternatives rejected

- Modifiers in insertion order: equipping the same items in another order gives another maximum.
- One maximum policy for all: either equipping heals, or a full stomach looks empty.
- Firing thresholds while the value is below them: a status display floods with repeats.
- Code that updates derived values at each change site: the one site nobody remembered goes stale.
- Recomputing derived values until nothing changes: costs a loop per change; the lag of a later
  declared input is documented by a spike instead.

## Would change if

Any decision test in `crates/lockstep-attributes/tests/decisions` fails: base 100 with +20 and ×1.5
must give 180; health 50 of 100 with +50 percent 75 of 150 and hunger 50 of 150; a drop of 70 through
60, 40 and 20 three ordered events; critical chance must track Luck, a relic's +2 and its removal;
corruption's minimum must never fall.
