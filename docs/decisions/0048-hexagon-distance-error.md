<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0048 The hexagon distance error is a recorded departure (decided)

Status: decided. It closes the hexagon item left open in decision 0005 (gap G1) by recording it as a departure.

## The finding

The reference's circle decision says a path around a circle of radius 50 deviates from the true distance by at most 3 percent for hexagons. No hexagon step count can do that.

Counting hexagon steps gives the true distance along the six lattice directions and over-counts everywhere else. In true geometry (odd rows half a cell to the right, rows 0.866 apart, every neighbour one unit away) the ratio of step distance to true distance, over every pair of points on the circle, runs from 1.0000 along an axis to 1.1547 (2 / sqrt 3) halfway between two axes. The worst error is therefore 15.47 percent.

It is not a matter of scale. Whatever constant a step is worth, the ratio still spans 1.0000 to 1.1547, and the best constant factor leaves 7.18 percent either way, more than twice the reference's 3 percent. A figure under 3 percent needs a distance that is not a step count (a Euclidean one), which would give up the integer, topology-neutral tenths that the pathfinder, flow field and `step_cost` share.

Squares compare as follows (decision 0005, `square8_for_rectangular_streets`): `Square8` 7.70 percent (reference: at most 8), `Square4` 41.42 percent (reference: at least 30), hexagons 15.47 percent (reference: at most 3). So hexagons are not more isotropic than `Square8` by this measure, and the reason to keep them is the one the reference gives for round shapes and six equal neighbours, not the distance figure.

## Decision

Hexagons keep the cube distance and the figure of 15.47 percent. The reference's 3 percent is recorded in the roadmap as a departure (the number cannot be met by any hexagon step count), not as an open gap waiting for an owner.

`tests/decisions/hexagon_error_is_a_departure.rs` pins the measurement:

- the worst error over the circle is between 15.40 and 15.55 percent;
- the best constant factor leaves at least 7 percent.

If either leaves its range, the distance or the geometry changed, and that is a design question.

## Alternatives

- Revise the reference's figure to 16 percent and keep the gap open: the finding is final, so a gap that can never close is a departure.
- A Euclidean hexagon distance: rejected above. It would also make costs differ per direction, which `step_cost` equal to `distance` for adjacent cells (decision 0005) rules out.
