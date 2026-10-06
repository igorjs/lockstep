// SPDX-License-Identifier: Apache-2.0
use crate::common::tally::{clock_configuration, step_configuration, Add, Honest, Start};
use lockstep_core::{replay, Recorder, Recording, ReplayError, ReplayOutcome, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn recorder(seed: u64, every: u64) -> Recorder<Honest> {
    Recorder::new(
        "tally",
        Start { total: 5 },
        seed,
        step_configuration(),
        clock_configuration(),
        every,
    )
}

#[test]
fn a_recorded_session_replays_to_the_same_hash() {
    let mut recording = recorder(1, 50);
    for step in 0..1_000 {
        let intents = if step % 7 == 0 {
            vec![Add(step)]
        } else {
            vec![]
        };
        recording.step_once(&intents);
    }
    let expected = recording.runner().hash();
    let recording = recording.into_recording();
    let (runner, outcome) = replay::<Honest>(&recording).unwrap();
    assert_eq!(outcome, ReplayOutcome::Identical);
    assert_eq!(runner.hash(), expected);
    assert_eq!(runner.step_number(), 1_000);
}

#[test]
fn replay_is_exact_over_fuzzed_frames_queues_and_multipliers() {
    for seed in 0..20 {
        let mut fuzz = Streams::new(seed);
        let mut recording = recorder(seed, 37);
        for _ in 0..600 {
            // Frames of 0 to 100 milliseconds, so a frame runs no step, one step, or several.
            let frame = fuzz.range("frame", 0, 100) as f32 / 1_000.0;
            if fuzz.range("queue", 0, 4) == 0 {
                let count = fuzz.range("count", 1, 4);
                recording.queue((0..count).map(|_| Add(fuzz.range("amount", -50, 50) as i64)));
            }
            if fuzz.range("multiplier", 0, 50) == 0 {
                let choices = [0.0, 0.25, 1.0, 3.7, 20.0, 1_000.0];
                recording.set_clock_multiplier(choices[fuzz.pick("which", choices.len())]);
            }
            recording.advance(frame);
        }
        let expected = recording.runner().hash();
        let recording = recording.into_recording();
        let (runner, outcome) = replay::<Honest>(&recording).unwrap();
        assert_eq!(outcome, ReplayOutcome::Identical, "seed {seed}");
        assert_eq!(runner.hash(), expected, "seed {seed}");
    }
}

#[test]
fn checkpoints_land_on_every_multiple_even_inside_one_advance() {
    let mut recording = recorder(2, 3);
    // A quarter second runs seven or eight steps in one call.
    for _ in 0..5 {
        recording.advance(0.25);
    }
    let steps: Vec<u64> = recording
        .recording()
        .checkpoints
        .iter()
        .map(|(step, _)| *step)
        .collect();
    let expected: Vec<u64> = (0..=recording.runner().step_number())
        .filter(|step| step % 3 == 0)
        .collect();
    assert!(recording.runner().step_number() >= 35);
    assert_eq!(steps, expected);
}

#[test]
fn a_recording_survives_its_bytes_and_bad_bytes_are_refused() {
    let mut recording = recorder(3, 10).keep_snapshots();
    for step in 0..100 {
        recording.step_once(&[Add(step)]);
    }
    let recording = recording.into_recording();
    assert_eq!(recording.snapshots.len(), recording.checkpoints.len());
    let bytes = recording.to_bytes().unwrap();
    let read = Recording::<Add>::from_bytes(&bytes).unwrap();
    assert_eq!(read, recording);
    assert!(matches!(
        Recording::<Add>::from_bytes(&bytes[..bytes.len() - 3]),
        Err(ReplayError::NotARecording(_))
    ));
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(matches!(
        Recording::<Add>::from_bytes(&longer),
        Err(ReplayError::NotARecording(_))
    ));
    let mut wrong = read;
    wrong.configuration = vec![1, 2, 3];
    assert!(matches!(
        replay::<Honest>(&wrong),
        Err(ReplayError::Configuration(_))
    ));
}

#[test]
fn checkpoints_that_could_never_be_compared_are_an_error() {
    let mut recording = recorder(4, 10);
    for step in 0..50 {
        recording.step_once(&[Add(step)]);
    }
    let recording = recording.into_recording();
    let mut past_the_end = recording.clone();
    past_the_end.checkpoints.push((500, 1));
    assert_eq!(
        replay::<Honest>(&past_the_end).err(),
        Some(ReplayError::Checkpoints { step: 500 })
    );
    let mut out_of_order = recording.clone();
    out_of_order.checkpoints.swap(2, 3);
    assert!(matches!(
        replay::<Honest>(&out_of_order),
        Err(ReplayError::Checkpoints { .. })
    ));
    let mut repeated = recording;
    let first = repeated.checkpoints[1];
    repeated.checkpoints.insert(1, first);
    assert!(matches!(
        replay::<Honest>(&repeated),
        Err(ReplayError::Checkpoints { .. })
    ));
}
