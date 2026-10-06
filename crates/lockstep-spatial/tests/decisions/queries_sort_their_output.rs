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

        // The same layout reached through churn: bodies wander, leave, and come back in another order.
        let mut churned = Occupancy::new(&map);
        let mut order: Vec<usize> = (0..who.len()).collect();
        for index in (1..order.len()).rev() {
            order.swap(index, streams.pick("shuffle", index + 1));
        }
        for index in &order {
            let detour = map.index(
                streams.range("x", 0, 30) as u32,
                streams.range("y", 0, 30) as u32,
            );
            let _ = churned.place(detour, who[*index]);
        }
        for index in order.iter().rev() {
            if streams.chance("leave", 0.5) {
                churned.vacate(who[*index]);
            }
        }
        for index in &order {
            churned.vacate(who[*index]);
        }
        for index in &order {
            churned.place(target[*index], who[*index]).unwrap();
        }

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
