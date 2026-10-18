// SPDX-License-Identifier: Apache-2.0
//! Decision: a hire's attributes are the archetype's base plus whole points bought above it, each
//! at its attribute's cost, within one budget; spending the budget exactly is allowed, one point
//! more is refused, and nothing is ever bought down for a refund.
//! Alternative rejected: free-form values checked against a total, which cannot say what an
//! archetype is, and refunds, which make the order of purchases matter.
//! Would change if: a purchase set costing more than the budget builds, or one costing exactly
//! the budget does not.

use crate::common::templates;
use lockstep_meta::TemplateError;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_budget_is_spent_exactly_and_no_further() {
    let templates = templates();
    assert!(templates.build("cashier", &[("judgement", 3)]).is_ok());
    assert_eq!(
        templates.build("cashier", &[("judgement", 3), ("stamina", 1)]),
        Err(TemplateError::OverBudget { cost: 7, budget: 6 })
    );
}
