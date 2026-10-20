// SPDX-License-Identifier: Apache-2.0
//! A batch of path requests, solved serially or on several threads with identical answers.
//!
//! This is the one place the project allows threads in simulation code, and the reason is that the
//! result cannot depend on them: every search is a pure function of the map, the bodies, the request
//! and the options, with integer arithmetic only. Each thread gets its own search buffers, every
//! answer lands in the slot of its request, and the caller applies the answers in request order. A
//! batch therefore gives the same answers on one thread or many, on every platform. The lint in
//! `scripts/lint-determinism.sh` names this file as the only exception.

use crate::cell::Cell;
use crate::jump::JumpPathfinder;
use crate::map::GridMap;
use crate::occupancy::Occupancy;
use crate::pathfinder::{PathOptions, PathResult, Pathfinder};
use crate::topology::{Square8, Topology};

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

fn solve_jump(
    pathfinder: &mut JumpPathfinder,
    map: &GridMap<Square8>,
    occupancy: &Occupancy,
    requests: &[PathRequest],
    options: PathOptions,
) -> Vec<PathAnswer> {
    requests
        .iter()
        .map(|request| {
            let mut path = Vec::new();
            let result =
                pathfinder.find(map, occupancy, request.from, request.to, options, &mut path);
            (result, path)
        })
        .collect()
}

/// `find_paths` with the jump search (decision 0077): the same results and costs as the plain
/// batch, in request order, on a thread pool with the `parallel` feature and serially without it.
/// The cells of equal-cost paths may differ from the plain batch (decision 0075). Returns `None`
/// when the ground is not uniform; use `find_paths` then.
pub fn find_jump_paths(
    map: &GridMap<Square8>,
    occupancy: &Occupancy,
    requests: &[PathRequest],
    options: PathOptions,
) -> Option<Vec<PathAnswer>> {
    let base = JumpPathfinder::new(map)?;
    #[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
    {
        use rayon::prelude::*;
        let chunk = requests
            .len()
            .div_ceil(rayon::current_num_threads() * 4)
            .max(1);
        let chunks: Vec<Vec<PathAnswer>> = requests
            .par_chunks(chunk)
            .map(|part| solve_jump(&mut base.fork(), map, occupancy, part, options))
            .collect();
        Some(chunks.into_iter().flatten().collect())
    }
    #[cfg(not(all(feature = "parallel", not(target_arch = "wasm32"))))]
    {
        let mut base = base;
        Some(solve_jump(&mut base, map, occupancy, requests, options))
    }
}
