# 0006 Threads for a batch of paths (decided)

Status: decided and implemented. The owner approved bending the "no threads in simulation code" rule
for this, on the condition that results stay deterministic and replayable.

## Decision

`lockstep-spatial` can solve a batch of path requests on a thread pool: `find_paths(map, occupancy,
requests, options)` returns one answer per request, in request order. With the `parallel` feature it
runs on `rayon`; without it, and always on WebAssembly, it runs serially. The answers are identical
either way, because:

- every search is a pure function of the map, the bodies, the request and the options, in integer arithmetic;
- each thread has its own search buffers, and nothing is shared that a search writes;
- every answer lands in its request's slot, and the caller applies the answers in request order.

A simulation calls it inside a step and applies the answers before the step ends, so replay and the
state hash do not change whether the batch ran on one thread or many.

This file is the only place threads are allowed. `scripts/lint-determinism.sh` names it as the one
exception, so a thread anywhere else still fails the build.

## Measured (Apple M1 Pro, 8 performance and 2 efficiency cores, release profile)

| Run | 500 paths on 512 by 512, 20 percent walls |
| --- | --- |
| Serial | about 610 to 650 milliseconds |
| Batch on the pool | about 87 to 89 milliseconds, the same answers |

The first version gave each piece of work its own search buffers (four megabytes on this map) and took
95 milliseconds. Chunking the requests, a few chunks per thread with one set of buffers each, brought it
to 87.

The reference's target of 50 milliseconds is still not met. Hierarchical search or a cheaper step would
close the rest. See the open item in decision 0005.

## Alternatives

- Standard library scoped threads with fixed chunks: no dependency, measured at 86 milliseconds, but it starts new threads every batch and balances uneven searches worse. `rayon` keeps one pool and steals work.
- A fixed-latency pipeline, where a request at step N is applied at exactly step N plus k while the search runs across frames: also deterministic, and worth adding if a host needs searches to overlap frames. Not built.
- `wasm-bindgen-rayon` for threads in the browser: it needs a nightly toolchain, atomics and cross-origin isolation, so the web build stays serial.

## Tests

`tests/decisions/threads_change_no_path.rs` checks that a pooled batch and a serial one hash the same over
50 maps of 64 requests. `just test` runs the spatial crate with the feature, so the pool really runs in
that test. The behaviour tests check every answer against a single search for its own request. The
benchmark's batch checksum must equal the serial one.
