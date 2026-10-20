<!-- SPDX-License-Identifier: Apache-2.0 -->

# 0077 The jump search runs as a batch on the thread pool (decided)

Status: decided and implemented (`find_jump_paths`). It closes gap G2 for exact paths on uniform ground with a thread pool.

## Decision

`find_jump_paths` is `find_paths` with `JumpPathfinder` (decisions 0006 and 0075): the map is scanned once, each chunk of requests gets a `fork` with fresh buffers, and answers land in request order, so they are the same on any pool size. It returns `None` on non-uniform ground. The only threads in simulation code stay in `batch.rs`.

## Measured (benchmark map, 500 paths, ten cores, best of five)

| Idea | Result | Kept |
| --- | --- | --- |
| Heap key packed into one integer | 371 to 317 milliseconds serial, same expansions | yes |
| Word-parallel scanning | no gain (decision 0075) | kept, harmless |
| Jump batch on the pool | 46 milliseconds, total cost asserted equal to the plain search | yes |
| Plain batch for comparison | 88 to 91 milliseconds | |
| Bucket open set, early target-region pruning | not built: after the batch passes the target the serial gain no longer decides G2 | no |

## What remains

A single thread takes 317 milliseconds, 6 times over 50; non-uniform ground and bodies as walls use the plain search. The 46 millisecond result depends on ten cores and has little margin.

## Tests

`tests/decisions/threads_change_no_path.rs`: the jump batch gives the plain results on every map, and the same cells on pools of 1, 3 and 8 threads. Benchmark asserts equal total cost.
