<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0017 Spoilage adds exact exposure, and a put never splits an item (decided)

Status: decided and implemented in milestone M7.

## Decision

- Spoilage adds up exposure as an exact integer: raw 16.16 game minutes times a whole percent for
  the temperature. An item spoils when the total reaches its minutes at 100 percent. The same game
  minutes spoil the same at any step rate.
- Units merged into one stack share the higher exposure: a stack spoils as its stalest unit.
  Spoiled and fresh units never share a stack, so a stack never turns spoiled without its own
  `Spoiled` event.
- A put merges an item into a stack only when every unit fits, and otherwise gives it a slot of
  its own or refuses. A put never splits an item, so one put changes at most one item.
- A binding affix keeps an item on its wearer. The affix can be removed while the item is worn,
  which is the repair that frees it.
- Containers and wearers are keyed by the simulation's own handles, and items live in a store of
  their own, so an item is an entity with its place recorded beside it.

## Alternatives rejected

- A freshness fraction lowered each step: it rounds once per step and drifts with the step rate.
- A weighted average of freshness when stacking: it hides a bad unit, and a split and merge
  changes freshness.
- Partial merges: one put would touch two items and could fail half way.
- Removing affixes only from unworn items: a bound item would stay on for ever.

## Would change if

A test in `crates/lockstep-inventory/tests/decisions/` fails: a day and a half at 30 and 60 steps
a second spoils differently, a half-spoiled ration leaves its stack fresher than half, four
rations beside a stack of four change the stack, or a sealed suit comes off before the repair.
