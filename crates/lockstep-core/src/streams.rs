use rand::{RngCore, SeedableRng};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Named, deterministic randomness.
///
/// Each name derives its seed from the master seed, so adding a stream, or calling one more
/// often, never changes another stream's sequence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Streams {
    master_seed: u64,
    streams: BTreeMap<String, Pcg32>,
}

impl Streams {
    pub fn new(master_seed: u64) -> Self {
        Self {
            master_seed,
            streams: BTreeMap::new(),
        }
    }

    fn stream(&mut self, name: &str) -> &mut Pcg32 {
        if !self.streams.contains_key(name) {
            let generator = Pcg32::seed_from_u64(self.master_seed ^ fnv1a(name));
            self.streams.insert(name.to_string(), generator);
        }
        self.streams
            .get_mut(name)
            .expect("the stream was inserted above")
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self, name: &str) -> f32 {
        (self.stream(name).next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// Integer in [low, high).
    pub fn range(&mut self, name: &str, low: i32, high: i32) -> i32 {
        assert!(low < high, "empty range");
        let width = (high as i64 - low as i64) as u32;
        (low as i64 + (self.stream(name).next_u32() % width) as i64) as i32
    }

    pub fn chance(&mut self, name: &str, probability: f32) -> bool {
        self.unit(name) < probability
    }

    /// Index in [0, length).
    pub fn pick(&mut self, name: &str, length: usize) -> usize {
        assert!(length > 0, "cannot pick from nothing");
        self.range(name, 0, length as i32) as usize
    }
}

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x100000001b3)
    })
}
