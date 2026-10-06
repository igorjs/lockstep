<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0023 Predicates are data, and everything fires once (decided)

Status: decided and implemented in milestone M9.

## Decision

- Predicates are a data tree read from JSON (`known`, `fragments`, `opened`, `fired`,
  `happened`, `all`, `any`, `not`), so a catalogue can be saved, compared and hashed, and its hash
  is the same on every platform.
- Learning a fact, opening a question and firing a rule each happen at most once per knower. What
  has happened is part of the saved state, so a save and load never fires anything again.
- A fact is known after fragments from distinct sources: the same source twice counts once.
- A rule may name only rules listed before it, so one pass in rule order settles them all.

## Alternatives rejected

- Predicates as closures: they cannot be saved, compared or hashed.
- Firing on each change into the true state: a condition that drops and returns would fire twice.
- Counting every fragment: one repeated source could prove anything.
- Rules naming any rule: firing would need repeated passes until nothing changes.

## Would change if

`crates/lockstep-knowledge/tests/decisions/` fails: a rule fires twice across a hundred
evaluations and a save and load, one source twice makes a two-fragment fact known, or the test
catalogue's hash changes without a commit naming the predicate rule that changed.
