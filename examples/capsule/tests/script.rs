use capsule::script::{parse, run, Command};
use capsule::{default_seed, default_steps, Cell, SCRIPT_TEXT};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn the_committed_script_parses_and_is_the_source_of_the_defaults() {
    let script = parse(SCRIPT_TEXT).unwrap();
    assert_eq!(
        (default_seed(), default_steps()),
        (script.seed, script.steps)
    );
    assert!(script.steps > 0);
    assert_eq!(script.commands.len(), 4);
    assert_eq!(
        script.commands[0],
        (
            10,
            Command::Move {
                cell: Cell { x: 15, y: 15 },
                run: false
            }
        )
    );
    assert_eq!(script.commands[3], (800, Command::Multiplier(20.0)));
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let script = parse("# a comment\n\nseed 5   # trailing\nsteps 9\n").unwrap();
    assert_eq!(
        (script.seed, script.steps, script.commands.len()),
        (5, 9, 0)
    );
}

#[test]
fn bad_lines_are_reported_with_their_line_number() {
    for (text, line) in [
        ("seed x", 1),
        ("steps 3\nwalk away", 2),
        ("1 move someone 1 1 walk", 1),
        ("1 move survivor 1 walk", 1),
        ("1 move survivor 1 1 hop", 1),
        ("1 dance", 1),
        ("1 multiplier fast", 1),
        ("seed 1\nseed 2", 2),
        ("steps 1\nsteps 2", 2),
        ("3 stop anyone", 1),
        ("3 stop", 1),
        ("10 move survivor 15 15 walk run", 1),
        ("seed 1 2", 1),
        ("5 multiplier 20 x", 1),
    ] {
        let error = parse(text).unwrap_err();
        assert_eq!(error.line, line, "{text:?}");
    }
}

#[test]
fn a_file_without_seed_or_steps_is_refused_for_the_whole_file() {
    assert_eq!(parse("steps 5\n").unwrap_err().line, 0);
    assert_eq!(parse("seed 5\n").unwrap_err().line, 0);
    assert!(
        parse("seed 5\nsteps 0\n").is_ok(),
        "zero steps is allowed when it is written down"
    );
}

#[test]
fn commands_at_one_step_apply_in_file_order() {
    let first = parse("seed 1\nsteps 5\n2 move survivor 3 3 walk\n2 stop survivor\n").unwrap();
    let second = parse("seed 1\nsteps 5\n2 stop survivor\n2 move survivor 3 3 walk\n").unwrap();
    assert_ne!(
        run(&first, None, None).hash(),
        run(&second, None, None).hash()
    );
}

#[test]
fn a_multiplier_applies_before_the_step_it_is_listed_on() {
    let early = parse("seed 1\nsteps 20\n5 multiplier 20\n").unwrap();
    let late = parse("seed 1\nsteps 20\n6 multiplier 20\n").unwrap();
    assert_ne!(
        run(&early, None, None).hash(),
        run(&late, None, None).hash()
    );
    assert_eq!(run(&early, None, None).clock().multiplier(), 20.0);
}

#[test]
fn overrides_replace_the_seed_and_the_step_count() {
    let script = parse(SCRIPT_TEXT).unwrap();
    let short = run(&script, Some(7), Some(50));
    assert_eq!(short.step_number(), 50);
    assert_ne!(short.hash(), run(&script, Some(8), Some(50)).hash());
}

#[test]
fn a_stop_command_cancels_a_walk() {
    let walking = parse("seed 1\nsteps 100\n1 move survivor 15 15 walk\n").unwrap();
    let stopped =
        parse("seed 1\nsteps 100\n1 move survivor 15 15 walk\n3 stop survivor\n").unwrap();
    let walking_runner = run(&walking, None, None);
    let stopped_runner = run(&stopped, None, None);
    let survivor = walking_runner.simulation().survivor().unwrap();
    let walked = walking_runner
        .simulation()
        .world()
        .positions
        .get(survivor)
        .copied();
    let halted = stopped_runner
        .simulation()
        .world()
        .positions
        .get(survivor)
        .copied();
    assert_ne!(walked, halted);
}
