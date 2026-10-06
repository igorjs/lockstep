//! Decision: randomness comes from named streams, each seeded from the master seed and its name.
//! Alternative rejected: one global generator.
//! Would change if: adding a thousand streams changes any existing stream (the number to beat
//! is zero changed draws).

use lockstep_core::Streams;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn a_thousand_other_streams_leave_one_stream_untouched() {
    let mut alone = Streams::new(8);
    let expected: Vec<i32> = (0..100)
        .map(|_| alone.range("loot", 0, 1_000_000))
        .collect();

    let mut crowded = Streams::new(8);
    for index in 0..1000 {
        crowded.unit(&format!("stream-{index}"));
    }
    let actual: Vec<i32> = (0..100)
        .map(|_| crowded.range("loot", 0, 1_000_000))
        .collect();
    assert_eq!(actual, expected);
}
