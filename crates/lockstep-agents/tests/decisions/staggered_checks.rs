// SPDX-License-Identifier: Apache-2.0
//! Decision: agent n checks its senses only on steps where `step % k == n % k`, n its slot, so
//! each step checks about one agent in k and every agent checks once every k steps.
//! Alternative rejected: every agent every step, which costs k times as much; and a round robin
//! from a counter, which reshuffles every time an agent is added or removed.
//! Would change if: with 1,000 agents and k = 10, any step checks more than 100 agents, or any
//! agent goes more than 10 steps without a check.

use lockstep_agents::checks_on;
use lockstep_core::StableVector;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_thousand_agents_checked_one_in_ten_cost_a_tenth_a_step() {
    let mut store = StableVector::new();
    let agents: Vec<_> = (0..1_000).map(|_| store.insert(())).collect();
    let every = 10;
    let mut last = vec![None::<u64>; agents.len()];
    for step in 0..100u64 {
        let mut checked = 0;
        for (index, agent) in agents.iter().enumerate() {
            if checks_on(*agent, step, every) {
                checked += 1;
                if let Some(previous) = last[index] {
                    assert_eq!(step - previous, 10);
                }
                last[index] = Some(step);
            }
        }
        assert!(checked <= 100, "step {step} checked {checked}");
    }
    assert!(last.iter().all(Option::is_some), "everyone checked");
}
