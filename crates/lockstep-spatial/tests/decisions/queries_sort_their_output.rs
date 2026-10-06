// SPDX-License-Identifier: Apache-2.0
//! Decision: every occupancy query sorts its output by cell, so results never depend on the order
//! bodies were placed, moved or removed.
//! Alternative rejected: returning bodies in slot order, which changes whenever a removal swaps the
//! last body into a freed slot.
//! Would change if: two occupancies holding the same bodies on the same cells ever answer `within`
//! differently (the number to beat is zero differences over 25 random histories).

use lockstep_core::{Handle, StableVector, Streams};
use lockstep_spatial::{Cell, GridMap, Occupancy, Square8};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn within_gives_the_same_answer_whatever_the_placement_history() {
    let map: GridMap<Square8> = GridMap::new(30, 30);
    let mut entities = StableVector::new();
    let who: Vec<Handle> = (0..40).map(|index| entities.insert(index)).collect();
    for seed in 0..25 {
        let mut streams = Streams::new(seed);
        // The final layout: body i stands on cell target[i].
        let target: Vec<Cell> = (0..who.len() as u32)
            .map(|index| Cell((index * 37) % 900))
            .collect();

        let mut straight = Occupancy::new(&map);
        for (handle, cell) in who.iter().zip(&target) {
            straight.place(*cell, *handle).unwrap();
        }

        // The same layout reached through churn: bodies land on detours outside the final layout,
        // some leave and come back while others are still placed (so removals really swap slots),
        // and the survivors walk to their final cells with move_to.
        let mut churned = Occupancy::new(&map);
        let mut order: Vec<usize> = (0..who.len()).collect();
        for index in (1..order.len()).rev() {
            order.swap(index, streams.pick("shuffle", index + 1));
        }
        let mut detour = || loop {
            let cell = Cell(streams.range("detour", 0, 900) as u32);
            if !target.contains(&cell) {
                return cell;
            }
        };
        for index in &order {
            let _ = churned.place(detour(), who[*index]);
        }
        for index in order.iter().step_by(3) {
            churned.vacate(who[*index]);
        }
        for index in order.iter().step_by(3) {
            let _ = churned.place(detour(), who[*index]);
        }
        for index in &order {
            if churned.cell_of(who[*index]).is_none() {
                churned.place(target[*index], who[*index]).unwrap();
            } else {
                churned.move_to(who[*index], target[*index]).unwrap();
            }
        }
        assert_eq!(
            churned, straight,
            "seed {seed}: the same bodies on the same cells"
        );

        let (mut a, mut b) = (Vec::new(), Vec::new());
        for radius in [0, 14, 40, 100, 240] {
            for centre in [map.index(0, 0), map.index(15, 15), map.index(29, 5)] {
                straight.within(&map, centre, radius, &mut a);
                churned.within(&map, centre, radius, &mut b);
                assert_eq!(a, b, "seed {seed}, radius {radius}");
                assert!(
                    a.windows(2).all(|pair| pair[0].0 < pair[1].0),
                    "sorted by cell"
                );
            }
        }
    }
}
