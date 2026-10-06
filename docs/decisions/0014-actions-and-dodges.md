<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0014 Actions commit, dodges are invulnerability, and the kernel owns timing (decided)

Status: decided and implemented in milestone M6.

## Decision

Each fighter is a state machine with phases (ready, wind-up, active, recovery, dodge startup,
dodge invulnerable) counted in whole steps: each duration in seconds becomes steps once, at the
simulation's steps per second. Counting down a rounded step length instead drifts (1/60 rounds
down in 16.16, which made a 0.2 second wind-up last 13 steps at 60 a second). The host animates to
the phases; it never decides when a hit lands.

- A strike happens on the first active step only, after every fighter has advanced. Every strike
  reads its targets before any resolves, so the lower handle's knockback cannot dodge the other's
  blow on the same step; the strikes then resolve in handle order.
- A perfect dodge refunds once: the window closes when used, and stamina never passes the maximum.
- A stagger interrupts a wind-up whose action allows it, never a recovery: committing is a real cost.
- Invulnerability is checked at the strike: an overlapping hit is dodged unless it grabs or cannot
  be avoided. A dodge within the perfect window of its press refunds stamina and opens a counter.
- The reference's cases are a dodge 0.11 seconds before the strike (perfect) and 0.13 (not),
  against a 0.12 second window. At 30 steps a second neither is a whole number of steps, so the
  test uses the steps either side of the window: 3 (0.100 seconds) and 4 (0.133 seconds).
- The counter window runs only while the fighter is free, so the dodge's own recovery does not eat
  it. The spec does not say when the window starts; starting it at the dodge left almost nothing.
- An order sent while busy waits up to 0.15 seconds; the newest order replaces a waiting one.
- The crate does not know about health: hits come back as events with their `DamageResult`, and the
  simulation applies them (to an attribute, usually). Stamina lives on the fighter until movement
  needs it too.

## Alternatives rejected

- Interrupting at any phase: an attack would commit to nothing.
- Measuring the perfect window from the start of invulnerability: the press is what a player times.
- Dropping orders sent while busy: input a frame early would be lost.
- Damage applied inside the crate: every simulation keeps health differently.

## Would change if

`tests/decisions/actions_and_dodges.rs` fails: an interrupt in wind-up but none in recovery, a
dodge and a grab through it, perfect at 3 steps and not at 4 (at 30 steps a second against a 0.12
second window), and a dodge pressed in recovery firing on the first free step.
