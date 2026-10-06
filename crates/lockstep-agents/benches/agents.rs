// SPDX-License-Identifier: Apache-2.0
//! Benchmark for the agents crate, with a committed baseline: 1,000 agents perceiving and
//! steering on a 256 by 256 map of half-metre cells with 10 percent walls.
//!
//! Five measurements, each the best of five runs:
//! - every agent looking for ten targets (no staggering, so the full cost),
//! - one 80 metre noise heard through a 10 metre a second wind,
//! - one `think` with a sighting for every agent and a budget of eight,
//! - one `steer` step of every agent down a flow field,
//! - one game day of weather, minute by minute.
//!
//! The baseline lives in `benches/baseline.txt` as `name microseconds` lines. The benchmark warns
//! when a measurement is more than 10 percent slower than its baseline, and never fails the build.
//! Run `just bench-baseline` after an intended change to write a new baseline.

use lockstep_agents::{
    hear, perceive, steer, think, Cone, Director, Mind, MindRules, Noise, Senses, SteerEvent,
    Stimulus, Surroundings, Weather, WeatherRules, Wind,
};
use lockstep_core::math::Fixed32;
use lockstep_core::{Column, Handle, StableVector, Streams};
use lockstep_spatial::{Cell, FlowField, GridMap, Occupancy, Square8};
use std::time::Instant;

const BASELINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benches/baseline.txt");

fn best_of_five(mut work: impl FnMut() -> u64) -> (u128, u64) {
    let mut best = u128::MAX;
    let mut check = 0;
    for _ in 0..5 {
        let start = Instant::now();
        check = work();
        best = best.min(start.elapsed().as_micros());
    }
    (best, check)
}

fn senses() -> Senses {
    Senses {
        sight: Cone {
            half_angle: 8_192,
            range_metres: Fixed32::from_int(12),
            around_metres: Fixed32::from_int(4),
        },
        hearing_range_metres: Fixed32::from_int(60),
        eye_height: 1,
    }
}

fn main() {
    let half = Fixed32::HALF;
    let mut streams = Streams::new(1);
    let mut map: GridMap<Square8> = GridMap::new(256, 256);
    for index in 0..map.cell_count() as u32 {
        if streams.range("wall", 0, 100) < 10 {
            map.set_passable(Cell(index), false);
        }
    }
    let mut occupancy = Occupancy::new(&map);
    let mut entities = StableVector::new();
    let mut place = |occupancy: &mut Occupancy, streams: &mut Streams| loop {
        let cell = Cell(streams.range("cell", 0, map.cell_count() as i32) as u32);
        if map.is_passable(cell) && occupancy.at(cell).is_none() {
            let who = entities.insert(());
            occupancy.place(cell, who).unwrap();
            return who;
        }
    };
    let agents: Vec<Handle> = (0..1_000)
        .map(|_| place(&mut occupancy, &mut streams))
        .collect();
    let targets: Vec<Handle> = (0..10)
        .map(|_| place(&mut occupancy, &mut streams))
        .collect();
    let mut column = Column::new();
    let mut minds = Column::new();
    for agent in &agents {
        column.set(*agent, senses());
        minds.set(*agent, Mind::default());
    }
    let facing = |who: Handle| (who.slot_index() as u16).wrapping_mul(6_553);

    let (perceiving, seen_check) = best_of_five(|| {
        let mut seen = Vec::new();
        perceive(
            &map, &occupancy, &column, &facing, &targets, 0, 1, half, &mut seen,
        );
        seen.len() as u64
    });

    let noise = Noise {
        at: occupancy.cell_of(targets[0]).unwrap(),
        loudness_metres: Fixed32::from_int(80),
        source: Some(targets[0]),
        psychic: false,
    };
    let wind = Wind {
        direction: 0,
        strength_metres_per_second: Fixed32::from_int(10),
    };
    let (hearing, heard_check) = best_of_five(|| {
        let mut heard = Vec::new();
        hear(&map, &occupancy, &column, &noise, wind, half, &mut heard);
        heard.len() as u64
    });

    let stimuli: Vec<(Handle, Stimulus)> = agents
        .iter()
        .enumerate()
        .map(|(index, agent)| {
            let target = targets[index % targets.len()];
            let at = occupancy.cell_of(target).unwrap();
            (*agent, Stimulus::Saw { target, at })
        })
        .collect();
    let rules = MindRules {
        forget_after_minutes: Fixed32::from_int(10),
        lose_sight_after_minutes: Fixed32::from_int(2),
        heard_confidence: half,
    };
    let (thinking, think_check) = best_of_five(|| {
        let mut fresh = minds.clone();
        let mut events = Vec::new();
        think(
            &mut fresh,
            &Column::new(),
            &stimuli,
            Fixed32::from_ratio(1, 30),
            &rules,
            &Director { alert_budget: 8 },
            &Surroundings {
                map: &map,
                occupancy: &occupancy,
                cell_metres: half,
            },
            &mut events,
        );
        events.len() as u64
    });

    let goal = occupancy.cell_of(targets[1]).unwrap();
    let field = FlowField::build(&map, &[goal], u32::MAX);
    // Each run steps a fresh copy, made before the clock starts.
    let mut copies: Vec<Occupancy> = (0..5).map(|_| occupancy.clone()).collect();
    let (steering, steer_check) = best_of_five(|| {
        let mut moved = copies.pop().expect("one copy a run");
        let mut events = Vec::new();
        steer(
            &map,
            &mut moved,
            &field,
            &agents,
            &Column::new(),
            half,
            &mut events,
        );
        events
            .iter()
            .filter(|event| matches!(event, SteerEvent::Stepped { .. }))
            .count() as u64
    });

    let weather_rules = WeatherRules {
        mean_strength: Fixed32::from_int(8),
        fronts_per_day: 3,
    };
    let (weathering, weather_check) = best_of_five(|| {
        let mut weather = Weather::new(0, Fixed32::from_int(4));
        let mut events = Vec::new();
        let mut streams = Streams::new(42);
        for _ in 0..1_440 {
            weather.advance(Fixed32::ONE, &weather_rules, &mut streams, &mut events);
        }
        weather.direction as u64 + events.len() as u64
    });

    let measurements = [
        ("perceive_1000_agents", perceiving),
        ("hear_1000_agents", hearing),
        ("think_1000_agents", thinking),
        ("steer_1000_agents", steering),
        ("weather_one_day", weathering),
    ];
    println!(
        "checksums (must not change): seen {seen_check}, heard {heard_check}, think {think_check}, steer {steer_check}, weather {weather_check}"
    );
    let write = std::env::args().any(|argument| argument == "--write-baseline");
    if write {
        let text: String = std::iter::once("# SPDX-License-Identifier: Apache-2.0\n".to_string())
            .chain(
                measurements
                    .iter()
                    .map(|(name, microseconds)| format!("{name} {microseconds}\n")),
            )
            .collect();
        std::fs::write(BASELINE, text).expect("write the baseline");
        println!("baseline written to {BASELINE}");
    }
    let baseline = std::fs::read_to_string(BASELINE).unwrap_or_default();
    for (name, microseconds) in measurements {
        let reference = baseline
            .lines()
            .find_map(|line| line.strip_prefix(name)?.trim().parse::<u128>().ok());
        match reference {
            Some(before) if microseconds * 100 > before * 110 => println!(
                "WARNING {name}: {microseconds} microseconds, {}% slower than the baseline {before}",
                (microseconds * 100 / before.max(1)) - 100
            ),
            Some(before) => println!("ok      {name}: {microseconds} microseconds (baseline {before})"),
            None => println!("new     {name}: {microseconds} microseconds (no baseline yet)"),
        }
    }
}
