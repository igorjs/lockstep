<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0056 A wave admits only what the budget leaves (decided)

Status: decided and implemented in milestone M8, closing deferral D5.

## Decision

- Each step the caller gives the wave a headroom, the budget less the current load
  (`Director::headroom`). The wave admits at most that many of the arrivals that have fallen due.
- Arrivals the budget turns away wait and are tried again each step. None is dropped, so a
  finished wave has admitted its whole total.
- A hold is reported once, when it starts, so a long hold is one event and not one per step.
- The caller owns the load. The wave never reads it, so the same wave serves queues, lanes or
  any other count.

## Alternatives rejected

- Dropping the excess: arrivals would be lost.
- Admitting a whole pulse and trusting the caller to cope: the budget the director exists to
  keep would be broken.

## Would change if

`waves_never_exceed_the_budget.rs` fails (the load passes the budget after a step, or a finished
wave admitted fewer than its total), or `the_queue_never_exceeds_the_budget_across_the_session`
fails in `examples/checkout/tests/checkout.rs`.
