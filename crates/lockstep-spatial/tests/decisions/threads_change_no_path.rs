// SPDX-License-Identifier: Apache-2.0
//! Decision: pathfinding may run a batch of searches on a thread pool (the `parallel` feature). It is
//! the one place threads are allowed in simulation code, because the answers cannot depend on them:
//! each search is a pure function of the map, the bodies and the request, each thread has its own
//! buffers, every answer lands in its request's slot, and the caller applies answers in request
//! order.
//! Alternative rejected: keeping every search on one thread, which left 500 paths on a 512 by 512
//! map at about 611 milliseconds; the rayon batch measured 87 to 89 milliseconds on ten cores.
//! Would change if: a threaded batch ever gives a different answer from the serial one (the number to
//! beat is zero differing answers over 50 maps of 64 requests), or a host needs paths on a schedule a
//! single batch per step cannot serve.

use crate::common::{free_cell, random_map};
use lockstep_core::Streams;
use lockstep_spatial::{
    find_paths, find_paths_serially, GridMap, Occupancy, PathOptions, PathRequest, Square8,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn batches() -> Vec<(GridMap<Square8>, Vec<PathRequest>)> {
    (200..250)
        .map(|seed| {
            let map: GridMap<Square8> = random_map(seed, 36, 36, 22);
            let mut streams = Streams::new(seed);
            let requests = (0..64)
                .map(|_| PathRequest {
                    from: free_cell(&map, &mut streams, "from"),
                    to: free_cell(&map, &mut streams, "to"),
                })
                .collect();
            (map, requests)
        })
        .collect()
}

#[test]
fn a_batch_gives_the_same_answers_on_one_thread_or_many() {
    for (index, (map, batch)) in batches().iter().enumerate() {
        let occupancy = Occupancy::new(map);
        let pooled = find_paths(map, &occupancy, batch, PathOptions::default());
        let serial = find_paths_serially(map, &occupancy, batch, PathOptions::default());
        assert_eq!(pooled, serial, "map {index}");
    }
}

/// The pool size changes how requests are chunked and which thread takes which chunk. None of it may
/// reach the answers.
#[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
#[test]
fn a_batch_gives_the_same_answers_on_pools_of_any_size() {
    let maps = batches();
    for threads in [1, 3, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        for (index, (map, batch)) in maps.iter().enumerate() {
            let occupancy = Occupancy::new(map);
            let pooled =
                pool.install(|| find_paths(map, &occupancy, batch, PathOptions::default()));
            let serial = find_paths_serially(map, &occupancy, batch, PathOptions::default());
            assert_eq!(pooled, serial, "{threads} threads, map {index}");
        }
    }
}

fn uniform_batches() -> Vec<(GridMap<Square8>, Vec<PathRequest>)> {
    batches()
        .into_iter()
        .map(|(map, requests)| {
            let mut flat: GridMap<Square8> = GridMap::new(map.width(), map.height());
            for index in 0..map.cell_count() as u32 {
                flat.set_passable(
                    lockstep_spatial::Cell(index),
                    map.is_passable(lockstep_spatial::Cell(index)),
                );
            }
            (flat, requests)
        })
        .collect()
}

/// Decision 0077: the jump batch answers the same on one thread or many, with the plain batch's
/// results and costs.
#[test]
fn a_jump_batch_gives_the_plain_results_and_costs_on_any_pool() {
    use lockstep_spatial::{find_jump_paths, JumpPathfinder};
    for (index, (map, batch)) in uniform_batches().iter().enumerate() {
        let occupancy = Occupancy::new(map);
        let plain = find_paths_serially(map, &occupancy, batch, PathOptions::default());
        let jump = find_jump_paths(map, &occupancy, batch, PathOptions::default()).unwrap();
        assert_eq!(jump.len(), plain.len());
        for ((got, _), (wanted, _)) in jump.iter().zip(&plain) {
            assert_eq!(got, wanted, "map {index}");
        }
        let mut single = JumpPathfinder::new(map).unwrap();
        let reused: Vec<_> = batch
            .iter()
            .map(|request| {
                let mut path = Vec::new();
                let result = single.find(
                    map,
                    &occupancy,
                    request.from,
                    request.to,
                    PathOptions::default(),
                    &mut path,
                );
                (result, path)
            })
            .collect();
        assert_eq!(jump, reused, "map {index}");
    }
}

#[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
#[test]
fn a_jump_batch_gives_the_same_cells_on_pools_of_any_size() {
    use lockstep_spatial::find_jump_paths;
    let maps = uniform_batches();
    let reference: Vec<_> = maps
        .iter()
        .map(|(map, batch)| {
            let occupancy = Occupancy::new(map);
            find_jump_paths(map, &occupancy, batch, PathOptions::default()).unwrap()
        })
        .collect();
    for threads in [1, 3, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();
        for (index, (map, batch)) in maps.iter().enumerate() {
            let occupancy = Occupancy::new(map);
            let answers = pool.install(|| {
                find_jump_paths(map, &occupancy, batch, PathOptions::default()).unwrap()
            });
            assert_eq!(answers, reference[index], "{threads} threads, map {index}");
        }
    }
}
