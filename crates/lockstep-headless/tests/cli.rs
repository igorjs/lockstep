use lockstep_headless::{run, CommandError};
use std::fs;

fn arguments(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

fn temporary_file(name: &str, contents: &str) -> String {
    let path =
        std::env::temp_dir().join(format!("lockstep-headless-{}-{name}", std::process::id()));
    fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn fixture_prints_a_sixteen_digit_hash_and_repeats() {
    let first = run(&arguments(&["fixture", "ledger", "--steps", "300"])).unwrap();
    let second = run(&arguments(&["fixture", "ledger", "--steps", "300"])).unwrap();
    assert_eq!(first.len(), 16);
    assert_eq!(first, second);
}

#[test]
fn the_seed_changes_the_hash() {
    let first = run(&arguments(&[
        "fixture", "capsule", "--seed", "1", "--steps", "100",
    ]))
    .unwrap();
    let second = run(&arguments(&[
        "fixture", "capsule", "--seed", "2", "--steps", "100",
    ]))
    .unwrap();
    assert_ne!(first, second);
}

#[test]
fn verify_accepts_the_matching_hash_and_names_a_divergence() {
    let hash = run(&arguments(&["fixture", "ledger", "--steps", "300"])).unwrap();
    let good = temporary_file("good", &format!("{hash}\n"));
    let bad = temporary_file("bad", "0000000000000000\n");

    let accepted = run(&arguments(&[
        "verify", "ledger", "--steps", "300", "--expect", &good,
    ]))
    .unwrap();
    assert_eq!(accepted, format!("ok ledger {hash}"));

    let rejected = run(&arguments(&[
        "verify", "ledger", "--steps", "300", "--expect", &bad,
    ]))
    .unwrap_err();
    assert_eq!(
        rejected,
        CommandError::Mismatch {
            name: "ledger".into(),
            expected: "0000000000000000".into(),
            actual: hash
        }
    );
}

#[test]
fn verify_tolerates_windows_line_endings() {
    let hash = run(&arguments(&["fixture", "ledger", "--steps", "50"])).unwrap();
    let file = temporary_file("crlf", &format!("{hash}\r\n"));
    assert!(run(&arguments(&[
        "verify", "ledger", "--steps", "50", "--expect", &file
    ]))
    .is_ok());
}

#[test]
fn bad_input_is_reported_not_panicked() {
    assert!(matches!(run(&arguments(&[])), Err(CommandError::Usage(_))));
    assert!(matches!(
        run(&arguments(&["fixture"])),
        Err(CommandError::Usage(_))
    ));
    assert!(matches!(
        run(&arguments(&["fixture", "nothing"])),
        Err(CommandError::UnknownFixture(_))
    ));
    assert!(matches!(
        run(&arguments(&["fixture", "ledger", "--steps", "many"])),
        Err(CommandError::Usage(_))
    ));
    assert!(matches!(
        run(&arguments(&["fixture", "ledger", "--steps"])),
        Err(CommandError::Usage(_))
    ));
    assert!(matches!(
        run(&arguments(&["fixture", "ledger", "--colour", "red"])),
        Err(CommandError::Usage(_))
    ));
    assert!(matches!(
        run(&arguments(&["verify", "ledger"])),
        Err(CommandError::Usage(_))
    ));
    assert!(matches!(
        run(&arguments(&["frobnicate", "ledger"])),
        Err(CommandError::Usage(_))
    ));
    assert!(matches!(
        run(&arguments(&[
            "verify",
            "ledger",
            "--expect",
            "/no/such/file"
        ])),
        Err(CommandError::Unreadable(_))
    ));
}
