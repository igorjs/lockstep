<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0019 Agents hunt a fading memory, and the director rations Alert (decided)

Status: decided and implemented in milestone M8.

## Decision

- An agent hunts its last known position, not its target. The memory's confidence fades linearly
  with its age in exact game minutes, from its starting confidence (1 for a sighting, less for a
  noise) to zero, and is then forgotten.
- Alert turns Searching once the target has been out of sight for the rules' minutes, and any
  agent whose memory is forgotten turns Idle. A noise never pulls an Alert agent away.
- The director grants Alert on one target to at most its budget of agents: nearest the target's
  last known position first, then those already Alert, then by handle. The rest stay Curious.
- A stimulus outside an agent's leash is ignored.

## Alternatives rejected

- Tracking the target itself: every agent would always know where it is.
- Lowering confidence by a fraction each step: it drifts with the step rate.
- First come first served for Alert: the agents that looked first would hold the budget while
  nearer ones wait.

## Would change if

`crates/lockstep-agents/tests/decisions/` fails: with 2 minutes to lose sight and 10 to forget an
agent turns Searching or Idle at any other time, at 30 or 60 steps a second; twenty agents
seeing one target with a budget of eight leave more than eight Alert, or not the eight nearest;
or a leashed agent reacts to a noise beyond its leash.
