// SPDX-License-Identifier: Apache-2.0
use crate::common::{free_cell, random_map};
use lockstep_core::Streams;
use lockstep_spatial::{
    find_paths, find_paths_serially, GridMap, Occupancy, PathOptions, PathRequest, PathResult,
    Pathfinder, Square8,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn requests(map: &GridMap<Square8>, seed: u64, count: usize) -> Vec<PathRequest> {
    let mut streams = Streams::new(seed);
    (0..count)
        .map(|_| PathRequest {
            from: free_cell(map, &mut streams, "from"),
            to: free_cell(map, &mut streams, "to"),
        })
        .collect()
}

#[test]
fn an_empty_batch_gives_no_answers() {
    let map: GridMap<Square8> = GridMap::new(8, 8);
    assert!(find_paths(&map, &Occupancy::new(&map), &[], PathOptions::default()).is_empty());
}

#[test]
fn every_answer_is_the_answer_to_its_own_request() {
    let map: GridMap<Square8> = random_map(3, 30, 30, 20);
    let occupancy = Occupancy::new(&map);
    let batch = requests(&map, 3, 40);
    let answers = find_paths(&map, &occupancy, &batch, PathOptions::default());
    assert_eq!(answers.len(), batch.len());
    let mut pathfinder = Pathfinder::new(&map);
    for (request, (result, path)) in batch.iter().zip(&answers) {
        let mut alone = Vec::new();
        let single = pathfinder.find(
            &map,
            &occupancy,
            request.from,
            request.to,
            PathOptions::default(),
            &mut alone,
        );
        assert_eq!((single, &alone), (*result, path), "{request:?}");
        if let PathResult::Found { .. } = result {
            if request.from == request.to {
                assert!(path.is_empty(), "a path to the same cell is empty");
            } else {
                assert_eq!(
                    path.last(),
                    Some(&request.to),
                    "a found path ends at the goal"
                );
            }
        }
    }
}

#[test]
fn the_batch_and_the_serial_batch_give_identical_answers() {
    for seed in 0..12 {
        let map: GridMap<Square8> = random_map(seed, 40, 40, 22);
        let occupancy = Occupancy::new(&map);
        let batch = requests(&map, seed, 64);
        assert_eq!(
            find_paths(&map, &occupancy, &batch, PathOptions::default()),
            find_paths_serially(&map, &occupancy, &batch, PathOptions::default()),
            "seed {seed}"
        );
    }
}
