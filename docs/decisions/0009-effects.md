<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0009 Effects tick by game minutes, with four stacking rules (decided)

Status: decided and implemented in milestone M4.

## Decision

Timed effects live in one `Effects` store for every entity, in the order they were applied, and
advance by game minutes, not by steps.

- A drain applies the change in its running total (`per_minute × minutes since applied`), not
  `per_minute × elapsed` per tick. The result depends only on the total minutes, so the frame rate
  and the clock multiplier cannot change it.
- Stacking is per tag on an entity: `Independent`, `RefreshDuration` (the modifier applies once, the
  timer restarts), `Replace`, and `DailyBudget` (the tag's total change on the entity is capped per
  game day; the excess is lost, not owed).
- `tick` takes the game day for budgets, and every call takes an `EffectContext` (attributes,
  registry and both event lists) instead of the four separate arguments the spec sketched.

## Alternatives rejected

- `per_minute × elapsed` per tick: one rounding per tick, so 1,800 short ticks drift from one long
  one.
- Diminishing returns as a factor per application: still rewards spamming and needs a tuned curve.
- Effects stored per entity: the spec's insertion order across entities would need a second index.

## Would change if

One minute in one tick and in 1,800 ticks gives different health or events; a refreshed effect
applies its modifier twice; a capped effect gives anything past its cap before the next day; or
twelve attributes, five effects and 1,000 ticks hash to anything but `fixtures/effects.hash` on any
platform.
