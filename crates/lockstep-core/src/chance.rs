//! Percentages as rolls. A chance crosses the roll boundary as basis points (10,000 is 100
//! percent), never as a float.

use crate::streams::Streams;
use serde::{Deserialize, Serialize};

/// Basis points in 100 percent.
pub const CERTAIN: u32 = 10_000;

/// A probability in basis points, clamped to `0..=10_000`.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Chance(pub u32);

impl Chance {
    pub const NEVER: Chance = Chance(0);
    pub const ALWAYS: Chance = Chance(CERTAIN);

    pub fn basis_points(points: u32) -> Self {
        Chance(points.min(CERTAIN))
    }

    pub fn percent(percent: u32) -> Self {
        Chance::basis_points(percent.saturating_mul(100))
    }

    fn clamped(self) -> u32 {
        self.0.min(CERTAIN)
    }
}

/// The memory of one smoothed roll: how many times in a row it has failed. Keep one per source
/// and save it with the entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SmoothedState {
    pub failures: u32,
}

impl Streams {
    /// A draw in basis points, uniform in `0..10_000`.
    fn basis_point_draw(&mut self, name: &str) -> u32 {
        ((self.next_u32(name) as u64 * CERTAIN as u64) >> 32) as u32
    }

    /// True with probability `chance`. Honest and streaky: use it for loot.
    pub fn roll(&mut self, name: &str, chance: Chance) -> bool {
        self.basis_point_draw(name) < chance.clamped()
    }

    /// Rolls `1 + |luck|` times. Positive luck keeps the best draw, negative luck the worst.
    /// Luck 3 at 25 percent lands about 68 percent. Display the nominal chance, always.
    pub fn roll_with_luck(&mut self, name: &str, chance: Chance, luck: i8) -> bool {
        let mut kept = self.basis_point_draw(name);
        for _ in 0..luck.unsigned_abs() {
            let draw = self.basis_point_draw(name);
            kept = if luck > 0 {
                kept.min(draw)
            } else {
                kept.max(draw)
            };
        }
        kept < chance.clamped()
    }

    /// A pseudo-random distribution: attempt n after a success succeeds with probability
    /// `n × increment`, and a success resets n. Same long-run rate, far fewer streaks.
    /// Use it for hits and criticals. Never use it for farmable rolls or for a roll that
    /// affects another player in multiplayer.
    pub fn roll_smoothed(&mut self, name: &str, chance: Chance, state: &mut SmoothedState) -> bool {
        // Draw first, as every roll does, so a chance that reaches 100 percent leaves the
        // stream's later draws where they were.
        let draw = self.next_u32(name) as u64;
        let points = chance.clamped();
        let attempt = state.failures as u64 + 1;
        let threshold = (smoothing_increment(points) as u64).saturating_mul(attempt);
        if points == CERTAIN || draw < threshold {
            state.failures = 0;
            true
        } else {
            state.failures = state.failures.saturating_add(1);
            false
        }
    }
}

const SMOOTHING_TABLE: &[u8] = include_bytes!("../fixtures/smoothing.bin");

/// Entries in `fixtures/smoothing.bin`: one per basis point, `0..=10_000`.
pub const SMOOTHING_ENTRIES: usize = CERTAIN as usize + 1;

/// The committed increment for a nominal chance, in units of 2^-32 per attempt.
pub fn smoothing_increment(points: u32) -> u32 {
    let index = points.min(CERTAIN) as usize * 4;
    u32::from_le_bytes(
        SMOOTHING_TABLE[index..index + 4]
            .try_into()
            .expect("the table holds 10,001 entries"),
    )
}

/// The long-run success rate of a smoothed roll with this increment (2^-32 units). It uses only
/// float addition, multiplication and division, which give the same bits on every platform.
#[doc(hidden)]
pub fn smoothed_rate(increment: u64) -> f64 {
    if increment == 0 {
        return 0.0;
    }
    // Expected attempts until a success: the sum over n of the chance that n - 1 attempts failed.
    let step = increment as f64 / 4_294_967_296.0;
    let mut survival = 1.0f64;
    let mut expected = 0.0f64;
    let mut attempt = 1u64;
    while survival > 1e-17 {
        expected += survival;
        let success = step * attempt as f64;
        if success >= 1.0 {
            break;
        }
        survival *= 1.0 - success;
        attempt += 1;
    }
    1.0 / expected
}

/// Searches for the increment whose long-run rate is nearest the nominal chance. This made
/// `fixtures/smoothing.bin`, and a test uses it to check the file.
#[doc(hidden)]
pub fn search_smoothing_increment(points: u32) -> u32 {
    let points = points.min(CERTAIN);
    if points == 0 {
        return 0;
    }
    if points == CERTAIN {
        return u32::MAX;
    }
    let target = points as f64 / CERTAIN as f64;
    // The smallest increment whose rate reaches the target. The rate rises with the increment.
    let (mut low, mut high) = (1u64, u32::MAX as u64);
    while low < high {
        let middle = low + (high - low) / 2;
        if smoothed_rate(middle) >= target {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let below = low - 1;
    if below >= 1 && target - smoothed_rate(below) <= smoothed_rate(low) - target {
        below as u32
    } else {
        low as u32
    }
}
