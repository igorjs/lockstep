// SPDX-License-Identifier: Apache-2.0

/// A small seeded generator (SplitMix64), so the harness needs no outside randomness.
pub struct SplitMix(u64);

impl SplitMix {
    pub fn new(seed: u64) -> Self {
        SplitMix(seed)
    }

    pub fn next_value(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    /// A number below `bound`, which must not be zero.
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next_value() % bound
    }
}

/// Values that sit on the edge of an integer's range or of a number's text.
const INTERESTING_NUMBERS: &[&str] = &[
    "0",
    "-0",
    "1",
    "-1",
    "255",
    "65535",
    "65536",
    "2147483647",
    "2147483648",
    "-2147483648",
    "-2147483649",
    "4294967295",
    "4294967296",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "18446744073709551616",
    "99999999999999999999999",
    "0.5",
    "1e999",
    "-1e999",
    "null",
    "\"\"",
    "[]",
    "{}",
];

const INTERESTING_BYTES: &[&[u8]] = &[
    &[0x00; 8],
    &[0xff; 8],
    &[0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    &[0x80, 0, 0, 0, 0, 0, 0, 0],
    &[0xff, 0xff, 0xff, 0x7f],
    &[0, 0, 0, 0x80],
    &[0x01],
    &[0xff],
];

/// One to four random edits of `base`: flip, replace, insert, delete, truncate, repeat, splice
/// from `other`, write an edge value, or replace a number in a text.
pub fn mutate(base: &[u8], other: &[u8], random: &mut SplitMix) -> Vec<u8> {
    let mut bytes = base.to_vec();
    for _ in 0..=random.below(4) {
        edit(&mut bytes, other, random);
        if bytes.len() > 1 << 20 {
            bytes.truncate(1 << 20);
        }
    }
    bytes
}

fn edit(bytes: &mut Vec<u8>, other: &[u8], random: &mut SplitMix) {
    let length = bytes.len();
    let at = if length == 0 {
        0
    } else {
        random.below(length as u64) as usize
    };
    match random.below(10) {
        0 if length > 0 => bytes[at] ^= 1 << random.below(8),
        1 if length > 0 => bytes[at] = random.next_value() as u8,
        2 => bytes.insert(at, random.next_value() as u8),
        3 if length > 0 => {
            let end = (at + 1 + random.below(16) as usize).min(length);
            bytes.drain(at..end);
        }
        4 => bytes.truncate(at),
        5 if length > 0 => {
            let end = (at + 1 + random.below(32) as usize).min(length);
            let chunk = bytes[at..end].to_vec();
            for _ in 0..=random.below(4) {
                bytes.splice(at..at, chunk.iter().copied());
            }
        }
        6 if !other.is_empty() => {
            let from = random.below(other.len() as u64) as usize;
            let end = (from + 1 + random.below(64) as usize).min(other.len());
            bytes.splice(at..at, other[from..end].iter().copied());
        }
        7 => {
            let value = INTERESTING_BYTES[random.below(INTERESTING_BYTES.len() as u64) as usize];
            let end = (at + value.len()).min(length);
            bytes.splice(at..end, value.iter().copied());
        }
        _ => replace_number(bytes, at, random),
    }
}

/// Replaces the run of digits at or after `at` with an edge value, or inserts one.
fn replace_number(bytes: &mut Vec<u8>, at: usize, random: &mut SplitMix) {
    let number = INTERESTING_NUMBERS[random.below(INTERESTING_NUMBERS.len() as u64) as usize];
    let start = (at..bytes.len())
        .find(|index| bytes[*index].is_ascii_digit())
        .unwrap_or(at.min(bytes.len()));
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    bytes.splice(start..end, number.bytes());
}
