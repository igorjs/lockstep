<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0024 Exclusions lock both ways, gates enforce once, keystones hold (decided)

Status: decided and implemented in milestone M10.

## Decision

- An exclusion written on either node of a pair locks both: taking one refuses the other, naming
  the node in the way, until the first is refunded.
- A gate is an attribute at or above a value, checked when the node is taken. An attribute that
  falls later takes nothing away.
- A keystone is never refunded, and the nodes it requires stay because it requires them.
- A refund returns the node's percent of its cost, rounded down, and removes its modifiers.
- Requirements may not loop, and a node no owner could take is refused: one whose full
  requirements hold two nodes that exclude each other, or a node it excludes.

## Alternatives rejected

- Exclusions only in the direction written: a forgotten other side lets an owner take both.
- Gates checked continuously: a node and its modifiers would flicker with the attribute.
- Refunding keystones at a loss: they would be ordinary nodes.

## Would change if

`crates/lockstep-progression/tests/decisions/` fails: an owner holding electrical takes
mechanical, or the reverse; senior is taken at 49 experience, refused at 50, or lost when
experience falls; or chief, or senior beneath it, is refunded.
