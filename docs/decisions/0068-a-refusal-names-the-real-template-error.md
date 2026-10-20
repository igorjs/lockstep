<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0068 A refused hire names the real template error (decided)

Status: decided and implemented in milestone M16.

## Decision

`roster` maps each `TemplateError` to the refusal that says what happened: `AboveMaximum` to
`Reason::AboveMaximum`, `OverBudget` to `Reason::OverBudget`, and any other error (an unknown
archetype or attribute, a malformed archetype) to the new `Reason::TemplateFailed`. Before, every
error but `AboveMaximum` was reported as over budget.

The scripted session now reaches `AboveMaximum`: one hire in eight buys four extra points of
craft, which takes a builder past the maximum. Each refusal (retired name, name in use, over
budget, above maximum) occurs in the fixture session, so the fixture hash covers all four.

## Alternatives rejected

- Keep one fallback refusal: it hides a broken template as a budget problem.
- Panic on an unexpected error: a hire is an intent from outside, and a refusal event is how the
  simulation reports a request it cannot honour.

## Would change if

`each_template_error_is_reported_as_itself` fails, or `the_session_reaches_every_hiring_refusal`
fails: the session stopped covering a refusal.
