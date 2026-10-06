//! A batch of path requests, solved serially or on several threads with identical answers.
//!
//! This is the one place the project allows threads in simulation code, and the reason is that the
//! result cannot depend on them: every search is a pure function of the map, the bodies, the request
//! and the options, with integer arithmetic only. Each thread gets its own search buffers, every
//! answer lands in the slot of its request, and the caller applies the answers in request order. A
//! batch therefore gives the same answers on one thread or many, on every platform. The lint in
//! `scripts/lint-determinism.sh` names this file as the only exception.

use crate::cell::Cell;
use crate::map::GridMap;
use crate::occupancy::Occupancy;
use crate::pathfinder::{PathOptions, PathResult, Pathfinder};
use crate::topology::Topology;

/// One search: from a cell to a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathRequest {
    pub from: Cell,
    pub to: Cell,
}

/// The answer to one request: the result and the cells to walk (excluding the start).
pub type PathAnswer = (PathResult, Vec<Cell>);

fn solve<T: Topology>(
    pathfinder: &mut Pathfinder,
    map: &GridMap<T>,
    occupancy: &Occupancy,
    request: &PathRequest,
    options: PathOptions,
) -> PathAnswer {
    let mut path = Vec::new();
    let result = pathfinder.find(map, occupancy, request.from, request.to, options, &mut path);
    (result, path)
}

/// Solves every request on the calling thread, in order.
pub fn find_paths_serially<T: Topology>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    requests: &[PathRequest],
    options: PathOptions,
) -> Vec<PathAnswer> {
    let mut pathfinder = Pathfinder::new(map);
    requests
        .iter()
        .map(|request| solve(&mut pathfinder, map, occupancy, request, options))
        .collect()
}

/// Solves every request and returns the answers in request order. With the `parallel` feature
/// (and outside WebAssembly) the searches run on a thread pool; otherwise they run serially. The
/// answers are identical either way.
pub fn find_paths<T: Topology + Sync>(
    map: &GridMap<T>,
    occupancy: &Occupancy,
    requests: &[PathRequest],
    options: PathOptions,
) -> Vec<PathAnswer> {
    #[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
    {
        use rayon::prelude::*;
        // A few chunks per thread, each with one set of search buffers (about four megabytes on a
        // 512 by 512 map, too much to build per request). Chunks are collected in index order and
        // flattened in order, so the answers follow the requests whichever thread finishes first.
        let chunk = requests
            .len()
            .div_ceil(rayon::current_num_threads() * 4)
            .max(1);
        let chunks: Vec<Vec<PathAnswer>> = requests
            .par_chunks(chunk)
            .map(|part| find_paths_serially(map, occupancy, part, options))
            .collect();
        chunks.into_iter().flatten().collect()
    }
    #[cfg(not(all(feature = "parallel", not(target_arch = "wasm32"))))]
    {
        find_paths_serially(map, occupancy, requests, options)
    }
}
