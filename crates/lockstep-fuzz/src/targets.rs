// SPDX-License-Identifier: Apache-2.0
use ledger::{Books, Configuration, Event, Intent, Ledger};
use lockstep_attributes::Registry;
use lockstep_calendar::Calendar;
use lockstep_core::math::{isqrt, Currency, Fixed32, Money};
use lockstep_core::{
    decode, encode, recorded_simulation_id, replay, Migrations, Recording, Runner, Save, Simulation,
};
use lockstep_crafting::Recipes;
use lockstep_inventory::Catalogue as InventoryCatalogue;
use lockstep_knowledge::Catalogue as KnowledgeCatalogue;
use lockstep_procgen::Templates;
use lockstep_progression::Graph;
use lockstep_regions::Regions;
use lockstep_relations::Kinds;

/// One thing to fuzz: a name that is also its corpus directory, and a function that takes
/// arbitrary bytes and panics when it meets a bug.
pub struct Target {
    pub name: &'static str,
    pub run: fn(&[u8]),
}

pub fn targets() -> Vec<Target> {
    vec![
        Target {
            name: "decode_recording",
            run: decode_recording,
        },
        Target {
            name: "decode_core_save",
            run: decode_core_save,
        },
        Target {
            name: "decode_save",
            run: decode_save,
        },
        Target {
            name: "decode_message",
            run: decode_message,
        },
        Target {
            name: "json_registry",
            run: |bytes| {
                let _ = Registry::from_json(&text(bytes));
            },
        },
        Target {
            name: "json_inventory_catalogue",
            run: |bytes| {
                let registry = Registry::from_json(cargo_bay::ATTRIBUTES_JSON).expect("committed");
                let _ = InventoryCatalogue::from_json(&text(bytes), &registry);
            },
        },
        Target {
            name: "json_recipes",
            run: |bytes| {
                let catalogue = bakery::catalogue();
                let _ = Recipes::from_json(&text(bytes), &catalogue);
            },
        },
        Target {
            name: "json_relations",
            run: |bytes| {
                let _ = Kinds::from_json(&text(bytes));
            },
        },
        Target {
            name: "json_templates",
            run: |bytes| {
                let _ = Templates::from_json(&text(bytes));
            },
        },
        Target {
            name: "json_knowledge",
            run: |bytes| {
                let _ = KnowledgeCatalogue::from_json(&text(bytes), audit::EVENT_KINDS);
            },
        },
        Target {
            name: "json_progression",
            run: |bytes| {
                let registry = Registry::from_json(training::ATTRIBUTES_JSON).expect("committed");
                let _ = Graph::from_json(&text(bytes), &registry);
            },
        },
        Target {
            name: "json_regions",
            run: |bytes| {
                let _ = Regions::from_json(&text(bytes));
            },
        },
        Target {
            name: "json_calendar",
            run: |bytes| {
                let _ = Calendar::from_json(&text(bytes));
            },
        },
        Target {
            name: "arithmetic_fixed",
            run: arithmetic_fixed,
        },
        Target {
            name: "arithmetic_money",
            run: arithmetic_money,
        },
    ]
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Reads numbers off a byte string, zero once it runs out, so any input is a valid script.
struct Reader<'a> {
    bytes: &'a [u8],
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0; N];
        let count = N.min(self.bytes.len());
        out[..count].copy_from_slice(&self.bytes[..count]);
        self.bytes = &self.bytes[count..];
        out
    }
    fn byte(&mut self) -> u8 {
        self.take::<1>()[0]
    }
    fn i32(&mut self) -> i32 {
        i32::from_le_bytes(self.take())
    }
    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.take())
    }
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.take())
    }
    fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

/// Decodes the bytes as a message and checks that what came out is stable: encoding it and
/// decoding that again gives the same bytes. A decode may tidy its input (a column drops trailing
/// empty slots), so the first encoding need not equal the input.
fn round_trips<T: serde::Serialize + serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    let value = decode::<T>(bytes).ok()?;
    let once = encode(&value);
    let again = decode::<T>(&once).expect("what was encoded decodes");
    assert_eq!(encode(&again), once, "encoding is stable once decoded");
    Some(value)
}

fn decode_recording(bytes: &[u8]) {
    let _ = recorded_simulation_id(bytes);
    if let Some(recording) = round_trips::<Recording<Intent>>(bytes) {
        let _ = replay::<Ledger>(&recording);
    }
}

fn decode_core_save(bytes: &[u8]) {
    if let Ok(save) = Save::from_bytes(bytes) {
        if let Ok(mut runner) = Runner::<Ledger>::from_save(&save, &Migrations::new()) {
            for step in 0..4 {
                runner.set_clock_multiplier(step as f32 * 1_000.0);
                runner.step_once(&[]);
            }
        }
    }
}

fn decode_save(bytes: &[u8]) {
    let Some(books) = round_trips::<Books>(bytes) else {
        return;
    };
    let _ = books.total_minor();
    let restored = Ledger::restore(books.clone());
    assert_eq!(restored.snapshot(), books);
}

fn decode_message(bytes: &[u8]) {
    round_trips::<Intent>(bytes);
    round_trips::<Event>(bytes);
    round_trips::<Configuration>(bytes);
    round_trips::<Money>(bytes);
    round_trips::<Currency>(bytes);
}

fn arithmetic_fixed(bytes: &[u8]) {
    let mut reader = Reader { bytes };
    // The first bytes pick a decimal text to parse, the rest are numbers to calculate with.
    let _ = Fixed32::parse_decimal(&text(&bytes[..bytes.len().min(24)]));
    while !reader.is_empty() {
        let a = Fixed32::from_raw(reader.i32());
        let b = Fixed32::from_raw(reader.i32());
        let one = Fixed32::ONE;
        assert_eq!(a + b, b + a);
        assert_eq!(a + b - b, a);
        assert_eq!(-(-a), a);
        assert_eq!(a * b, b * a);
        assert_eq!(a * one, a);
        assert_eq!(a / one, a);
        if b != Fixed32::ZERO {
            let _ = a / b;
        }
        let low = a.min(b);
        let high = a.max(b);
        assert!(low <= high);
        assert_eq!(a.clamp(low, high), a);
        // Floor and round bracket the value.
        let raw = a.raw() as i64;
        assert!(((a.floor() as i64) << 16) <= raw && raw < (((a.floor() as i64) + 1) << 16));
        assert!((a.round() as i64 - (raw >> 16)).abs() <= 1);
        let _ = a.lerp(b, Fixed32::HALF);
        let _ = a.abs();
        if a.raw() >= 0 {
            let root = a.sqrt().raw() as i128;
            let target = (a.raw() as i128) << 16;
            assert!(root * root <= target && target < (root + 1) * (root + 1));
        }
        // from_ratio mirrors for a negated numerator, except where negation wraps.
        let (numerator, denominator) = (a.raw(), b.raw());
        if denominator != 0 && numerator != i32::MIN {
            assert_eq!(
                Fixed32::from_ratio(-numerator, denominator),
                -Fixed32::from_ratio(numerator, denominator)
            );
        }
        let wide = reader.u64();
        let root = isqrt(wide) as u128;
        assert!(root * root <= wide as u128 && (wide as u128) < (root + 1) * (root + 1));
    }
}

fn arithmetic_money(bytes: &[u8]) {
    let mut reader = Reader { bytes };
    let code = [
        b'A' + reader.byte() % 26,
        b'A' + reader.byte() % 26,
        b'A' + reader.byte() % 26,
    ];
    let exponent = reader.byte() % 20;
    let other_exponent = (exponent + 1) % 19;
    let (Ok(currency), Ok(other)) = (
        Currency::new(std::str::from_utf8(&code).expect("ASCII"), exponent),
        Currency::new(std::str::from_utf8(&code).expect("ASCII"), other_exponent),
    ) else {
        return;
    };
    while !reader.is_empty() {
        let a = Money::from_minor(reader.i64(), currency);
        let b = Money::from_minor(reader.i64(), currency);
        let _ = a.to_string();
        assert_eq!(a.checked_add(b), b.checked_add(a));
        if let Ok(sum) = a.checked_add(b) {
            assert_eq!(sum.checked_sub(b), Ok(a));
        }
        assert!(a.checked_add(Money::from_minor(0, other)).is_err());
        let _ = a.checked_mul(reader.i64());
        let _ = Money::from_major(reader.i64(), currency);
        let count = (reader.byte() % 12) as usize;
        let weights: Vec<u64> = (0..count)
            .map(|_| match reader.byte() % 4 {
                0 => 0,
                1 => reader.byte() as u64,
                2 => reader.u64(),
                _ => u64::MAX,
            })
            .collect();
        match a.allocate(&weights) {
            Ok(parts) => {
                assert_eq!(parts.len(), weights.len());
                let sum: i128 = parts.iter().map(|part| part.minor() as i128).sum();
                assert_eq!(sum, a.minor() as i128, "the parts add up to the amount");
                let weight_sum: i128 = weights.iter().map(|weight| *weight as i128).sum();
                for (part, weight) in parts.iter().zip(&weights) {
                    let exact = a.minor() as i128 * *weight as i128;
                    // The check itself needs 130 bits for the extremes, so skip those.
                    if let Some(scaled) = (part.minor() as i128).checked_mul(weight_sum) {
                        assert!((scaled - exact).abs() < weight_sum);
                    }
                    assert!(*weight != 0 || part.minor() == 0);
                    assert!(part.minor().signum() * a.minor().signum() >= 0);
                }
            }
            Err(_) => assert!(weights.iter().all(|weight| *weight == 0)),
        }
        if let Ok(parts) = a.split(reader.byte() as u32 % 50 + 1) {
            let total = Money::checked_sum(parts.iter().copied(), currency);
            assert_eq!(total, Ok(a));
            // Parts differ by at most one minor unit.
            let largest = parts.iter().map(|part| part.minor()).max().unwrap();
            let smallest = parts.iter().map(|part| part.minor()).min().unwrap();
            assert!(largest as i128 - smallest as i128 <= 1);
        }
    }
}
