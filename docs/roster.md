<!-- SPDX-License-Identifier: Apache-2.0 -->

# roster (example)

The consumer scenario for `lockstep-meta`, and not a game. A crew is staffed for a project, and the
next project remembers who has left.

- Hiring takes a name from the pool of 16, a role (planner or builder) and points to buy above the
  role's base. The roles and the budget are in `data/templates.json`: craft and reliability, a
  budget of 5, craft costing 1 a point and reliability 2. A hire past the budget or a maximum is
  refused.
- The first member hired leads. When the lead leaves, `Control::lose` passes the lead to the first
  member of the hire order who remains; with none left the lead is vacant.
- A member who leaves has the name retired. Hiring a retired name, or a name already held, is
  refused.
- The host folds a finished project into the `Chronicle` with `fold`: each name that left is
  retired for good and departures are counted. `configuration_from` hands the retired names to the
  next project. In between, the chronicle goes through `seal` and `open`, as a save would.

A hire refused for a template error is reported as that error: over budget, above a maximum, or
`TemplateFailed` for anything else (decision 0068).

The fixture hash in `fixtures/roster.hash` covers two projects of 120 steps with random hires and
departures, the second starting from the chronicle the first left. It is the hash of the two
projects' final states. It is checked by `lockstep-headless verify roster` and by a WebAssembly
test. The session reaches every refusal: a retired name, a name in use, over budget and above a
maximum.

`lockstep-headless record roster` writes the two projects as a recording set, labelled `first` and
`second`. The second project's recorded configuration holds the retired names, so a replay rebuilds
both projects without the chronicle (decision 0067).

The milestone gate is `a_retired_name_stays_retired_in_the_next_project`: a name that left in the
first project is refused in the second after the chronicle was saved and opened. Other tests: the
lead falling back in hire order and never to the one who left, duplicate and retired names, the
point-buy budget and maximum, a damaged save, each template error reported as itself, the session hiring in both projects and
reaching every refusal, and the recorded set replaying to the fixture hash.

This example stands in for the reference's lineage consumer (departure P26 in the roadmap).
See decision 0031.
