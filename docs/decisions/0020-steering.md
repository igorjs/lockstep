<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0020 Steering reserves through occupancy, one agent at a time (decided)

Status: decided and implemented in milestone M8.

## Decision

- Agents steer down a flow field in handle order, one cell per call. Each moves through occupancy
  as it decides, so the cell is reserved at once: no later agent in the step can take it, and two
  bodies never share a cell.
- When the field's next cell is held, the agent tries the two neighbours either side of its way,
  nearest the intended direction first, before waiting. Agents and fighters share the one
  function, `lockstep_combat::sidestep`.
- A cell outside an agent's leash is never taken, except one no farther from home than the
  agent's own, so an agent knocked outside finds its way back. The sidestep filters by the leash
  while it chooses, so the other side is still tried.

## Alternatives rejected

- Deciding every move first and applying them together: it needs a separate pass to settle two
  agents wanting one cell.
- Waiting until the way clears: one standing body locks a crowd behind it.
- A sidestep of agents' own: the reference asks for one function shared with player movement.

## Would change if

`crates/lockstep-agents/tests/decisions/` fails: 200 agents crossing a one-cell doorway share a
cell or are not all through in 2,000 steps, or a mover with a free diagonal is not past a body
in its way within two steps.
