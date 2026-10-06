<!-- SPDX-License-Identifier: Apache-2.0 -->

# sparring (example)

The consumer scenario for `lockstep-combat`. Partners spar in pairs on one floor with a few
pillars: the first partner with the second, the third with the fourth, and so on.

- Each partner has a stick (`STICK`: a 0.2 second wind-up, a jab at the cell ahead that a stagger
  can interrupt), a sweep (`SWEEP`: a 0.45 second wind-up, an arc that staggers and knocks back a
  cell), a dodge of two cells, and a soft ball that flies six cells at two a step. Throwing costs 8
  stamina and needs a ready thrower; walking and running spend the same stamina pool.
- Each step runs, in order: the intents, `step_combat`, `step_movement` (a partner that is not
  ready stands still), `step_projectiles`, then the damage.
- The simulation applies the damage: each `Landed` hit and `ProjectileHit` takes its dealt amount
  from health, floored at zero. A partner knocked into a pillar or the edge of the floor takes 5
  more (`Impact`). A body knocked into another body takes nothing more.
- A partner whose health reaches zero yields: the point goes to the one who dealt the blow, and
  the health comes back in full.
- `coach` is the scripted opponent for both sides. A ready partner closes on its pair (running
  from more than six cells, walking nearer); next to it, it strikes, sweeps, dodges a wind-up it
  sees coming, or backs off below 30 stamina; from up to six cells it now and then throws. The
  coach draws from its own streams, so a session depends only on the seed.
- A restored snapshot continues exactly like the simulation that kept running, although the
  restored one starts with an empty path cache.

The fixture hash in `fixtures/sparring.hash` covers 6,000 steps of the duel (two partners on a
sixteen by nine floor), with dodges, strikes, throws and yields on both sides. It is checked by
`lockstep-headless verify sparring` and by a WebAssembly test.

The spike in `tests/spikes` has 200 partners spar for 10,000 steps and checks every step that no
two bodies share a cell and no health goes below zero. It takes about three seconds optimised and
a minute without, so `just test` runs it with `--release` and a plain `cargo test` skips it.

This example stands in for the reference's survivor against an opponent: two neutral partners,
so the survivor example stays free of combat (departure P15 in the roadmap).
