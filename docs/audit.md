<!-- SPDX-License-Identifier: Apache-2.0 -->

# audit (example)

The consumer scenario for `lockstep-knowledge`, and not a game. Eight accounts move money and an
auditor reviews every transfer as it happens.

- Each transfer may give the auditor fragments: a round amount (a whole hundred in minor units)
  is a fragment of `round_amounts`, a transfer before minute 300 of the day one of
  `night_transfers`, and a receiver paid by three or more distinct senders one of
  `shared_receiver`, sourced by the receiver so it counts once. Each fact needs two.
- The question `who_is_behind_it` opens after two fragments across those facts.
- The findings are rules in `data/knowledge.json`: `suspicion` (a revelation) when round amounts
  and night transfers are both known, `ring` (a secret) after suspicion once a shared receiver is
  known, `thorough` (an achievement) after 100 reviews, and `nothing_to_see` after 20 reviews
  while the question stays closed.
- Each review is a `Reviewed` event naming the auditor; `notice` counts them for `happened`.
- The world stores the catalogue's hash. `Audit::load` refuses a save made with another
  catalogue with `OtherCatalogue`; `restore` panics on one.

The fixture hash in `fixtures/audit.hash` covers 60,000 steps (2,000 game minutes, about 1.4 days)
of transfers between random accounts, one in eight a round amount. It is checked by
`lockstep-headless verify audit` and by a WebAssembly test.

The milestone gate is `the_first_question_opens_after_two_fragments`: across the session the
question opens on the step its second fragment arrives, and not before; and
`nothing_fires_twice_across_a_save` saves and loads half way and runs on to twice the length:
`suspicion`, `ring` and `thorough` each fire exactly once, and `nothing_to_see` never does,
since the question opens early (it has its own test). Other tests: a clean day earning `nothing_to_see`, invalid transfers refused,
and a save from another catalogue refused at load.

This example stands in for the reference's knowledge consumer, the first question of a question
ledger in the survivor's world (departure P20 in the roadmap).
