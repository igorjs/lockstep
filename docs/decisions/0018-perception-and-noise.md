<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0018 Wind scales a noise, and the threshold comes off after (decided)

Status: decided and implemented in milestone M8.

## Decision

- A noise carries `loudness × (1 + 0.04 × strength × cos θ)` metres, in exact integers rounded
  once, so the reference's 8.4, 3.6 and 6.0 metres hold exactly.
- The wind raises every listener's hearing threshold by 0.3 metres per metre a second, and a
  listener hears a noise within the carried range less that threshold. A 14 metre a second gale
  leaves a 6 metre footstep 1.8 metres across the wind and nothing against it.
- A psychic noise ignores the wind both ways. Walls still muffle it.
- A wall or a rise between source and listener halves the reach, traced from the listener at its
  own eye height.
- Agent n checks its senses on steps where `step % k == n % k`, n its handle's slot.

## Alternatives rejected

- Taking the threshold off the loudness before the wind scales it: the reference's 8.4, 3.6 and
  6.0 metre ranges would no longer hold.
- A gale that silences every footstep: footsteps run from 4 to 9 metres, and no single threshold
  silences them all, so the reference's "a gale masks walking" is read as walking being heard
  only within 2 metres across a gale (departure P17).
- Every agent checking every step: k times the cost.
- A round robin from a counter: adding or removing an agent reshuffles everyone's turn.

## Would change if

`crates/lockstep-agents/tests/decisions/` fails: the 6 metre footstep at 10 metres a second
carries anything but exactly 8.4, 3.6 and 6.0 metres, a crosswind footstep in a 14 metre a second
gale is heard beyond 2 metres, or a step checks more than one agent in k.
