<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0076 Exact paths still miss the 50 millisecond target (decided)

Status: gap G2 stays open for exact paths.

## Decision

The best exact search measured is the jump search of decision 0075: about 370 milliseconds for the benchmark's 500 paths, 7 times over the 50 millisecond target (the plain search is 611 to 690). The target is not weakened. It is still met only by the approximate hierarchical search (decision 0047, departure P47), which is 23 milliseconds with paths 6.4 percent dearer.

## What would be needed

At 126 nanoseconds per expanded jump point, 50 milliseconds for 500 paths allows about 800 jump points per path; the jump search expands about 5,900 and the plain search about 12,700 cells. The estimate is the cause: the cheapest path costs about 1.15 times the straight estimate, so the search fills a wide ellipse. Only a precomputed exact structure that answers distances (contraction hierarchy, hub labels, or exact distances to many landmarks) could close it. None was built; the size and query cost on a map this full of walls are doubtful, and decision 0047 measured landmarks at only 1.4 times fewer expansions.

## Tests

None beyond decision 0075: this records a measurement. The roadmap row G2 carries the numbers and `benches/baseline.txt` carries `paths_500_jump`.
