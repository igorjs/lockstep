<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0007 Chance in basis points, and smoothed rolls from a committed table (decided)

Status: decided and implemented in milestone M4.

## Decision

A percentage crosses the roll boundary as basis points (`Chance`, where 10,000 is 100 percent). Each
roll turns a raw 32-bit draw into basis points with a multiply and a shift, and compares integers.

The smoothed roll is a pseudo-random distribution: attempt n after a success succeeds with
probability n times an increment. No formula gives the increment for a nominal chance, so a search
finds it once for every basis point. The results are committed in
`crates/lockstep-core/fixtures/smoothing.bin` (10,001 little-endian `u32` values in units of 2^-32).
The search uses only float addition, multiplication and division, which give the same bits on every
platform, and a decision test runs it again and compares every entry (a spread of entries under
WebAssembly).

## Alternatives rejected

- Float probabilities: 0.1 plus 0.2 is not 0.3, and a chance computed on one platform could round
  to a different roll on another.
- Searching at run time: a search for every new chance, inside a step.
- A coarser table (whole percents) with interpolation: the rate between entries would drift from
  nominal.

## Numbers

- Every committed increment lands within 0.001 percent of its nominal rate.
- Smoothed 70 percent never misses five times in a row. Over 400,000 rolls, smoothed rolls at 1, 5,
  10, 25, 50 and 90 percent stay within half a percent of nominal and have shorter droughts than
  plain rolls.
- Luck 3 at 25 percent lands between 66 and 70 percent over 200,000 rolls.

## Would change if

A regenerated entry differs from the committed one on any platform, or a design needs a chance finer
than 0.01 percent.
