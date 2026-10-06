use lockstep_core::math::{
    atan2, cos, generate_atan_octant, generate_quarter_sine, isqrt, sin, unit, unit_circle_table,
    Fixed32, Vector2,
};
use lockstep_core::{hash_of, Streams};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn fixed(value: i32) -> Fixed32 {
    Fixed32::from_int(value)
}

// ------------------------------------------------------------------ Fixed32

#[test]
fn whole_numbers_and_ratios_convert_exactly() {
    assert_eq!(fixed(3).raw(), 3 << 16);
    assert_eq!(Fixed32::from_ratio(1, 2), Fixed32::HALF);
    assert_eq!(
        Fixed32::from_ratio(1, 3).raw(),
        21_845,
        "one third rounds to nearest"
    );
    assert_eq!(Fixed32::from_ratio(-1, 3).raw(), -21_845);
    assert_eq!(
        Fixed32::from_ratio(-1, 2),
        -Fixed32::HALF,
        "an exact negative half is exact"
    );
    assert_eq!(Fixed32::from_ratio(1, -2), -Fixed32::HALF);
    assert_eq!(Fixed32::from_ratio(-1, -2), Fixed32::HALF);
    assert_eq!(Fixed32::from_ratio(-3, 4).raw(), -49_152);
    assert_eq!(Fixed32::from_ratio(-5, 1), fixed(-5));
    assert_eq!(Fixed32::ONE.raw(), 65_536);
}

#[test]
fn a_negative_ratio_is_the_mirror_of_the_same_positive_one() {
    for numerator in 1..200 {
        for denominator in 1..40 {
            assert_eq!(
                Fixed32::from_ratio(-numerator, denominator),
                -Fixed32::from_ratio(numerator, denominator),
                "{numerator}/{denominator}"
            );
            assert_eq!(
                Fixed32::from_ratio(numerator, -denominator),
                Fixed32::from_ratio(-numerator, denominator)
            );
        }
    }
}

#[test]
fn floor_and_round_agree_with_their_definitions() {
    assert_eq!(Fixed32::from_raw(3 * 65_536 + 32_767).round(), 3);
    assert_eq!(Fixed32::from_raw(3 * 65_536 + 32_768).round(), 4);
    assert_eq!(Fixed32::from_raw(-1).floor(), -1);
    assert_eq!(Fixed32::from_raw(-65_536).floor(), -1);
    assert_eq!(Fixed32::from_raw(65_535).floor(), 0);
}

#[test]
fn arithmetic_matches_exact_values_and_wraps_on_overflow() {
    assert_eq!(fixed(2) * Fixed32::from_ratio(3, 2), fixed(3));
    assert_eq!(fixed(3) / fixed(2), Fixed32::from_ratio(3, 2));
    assert_eq!(-fixed(5) + fixed(5), Fixed32::ZERO);
    assert_eq!(
        Fixed32::from_raw(i32::MAX) + Fixed32::from_raw(1),
        Fixed32::from_raw(i32::MIN)
    );
    assert_eq!(
        Fixed32::from_raw(i32::MIN).abs(),
        Fixed32::from_raw(i32::MIN),
        "abs wraps, never panics"
    );
    assert_eq!(-Fixed32::from_raw(i32::MIN), Fixed32::from_raw(i32::MIN));
    let mut total = fixed(1);
    total += fixed(2);
    total -= fixed(1);
    assert_eq!(total, fixed(2));
}

#[test]
fn comparisons_clamp_and_blend() {
    assert_eq!(fixed(5).clamp(fixed(0), fixed(3)), fixed(3));
    assert_eq!(fixed(-5).clamp(fixed(0), fixed(3)), fixed(0));
    assert_eq!(fixed(2).min(fixed(7)), fixed(2));
    assert_eq!(fixed(2).max(fixed(7)), fixed(7));
    assert_eq!(fixed(10).lerp(fixed(20), Fixed32::HALF), fixed(15));
    assert_eq!(fixed(1).to_f32_for_display(), 1.0);
}

#[test]
fn perfect_squares_have_exact_square_roots_up_to_one_hundred_eighty() {
    for root in 0..=180 {
        assert_eq!(fixed(root * root).sqrt(), fixed(root), "root {root}");
    }
}

fn sqrt_samples() -> Vec<i32> {
    let mut streams = Streams::new(7);
    (0..100_000)
        .map(|_| streams.range("sqrt", 0, i32::MAX))
        .collect()
}

#[test]
fn square_roots_are_within_one_raw_unit_of_the_true_value_on_one_hundred_thousand_samples() {
    for raw in sqrt_samples() {
        let actual = Fixed32::from_raw(raw).sqrt().raw() as i64;
        let expected = ((raw as f64) * 65_536.0).sqrt().floor() as i64;
        assert!(
            (actual - expected).abs() <= 1,
            "sqrt of raw {raw}: {actual} against {expected}"
        );
    }
}

#[test]
#[should_panic(expected = "negative")]
fn the_square_root_of_a_negative_number_panics() {
    let _ = fixed(-1).sqrt();
}

#[test]
fn the_integer_square_root_is_a_floor_for_edge_values() {
    for (value, root) in [
        (0u64, 0u64),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 2),
        (15, 3),
        (16, 4),
        (u64::MAX, 4_294_967_295),
    ] {
        assert_eq!(isqrt(value), root, "isqrt({value})");
    }
}

// ------------------------------------------------------------------ Vector2

#[test]
fn length_is_exact_for_a_three_four_five_triangle_and_for_huge_vectors() {
    assert_eq!(Vector2::new(fixed(3), fixed(4)).length(), fixed(5));
    assert_eq!(Vector2::new(fixed(-3), fixed(4)).length(), fixed(5));
    let big = Vector2::new(fixed(20_000), fixed(0));
    assert_eq!(
        big.length(),
        fixed(20_000),
        "the 64-bit path does not overflow where x * x would"
    );
    assert_eq!(Vector2::ZERO.length(), Fixed32::ZERO);
}

#[test]
fn a_length_too_long_for_fixed_point_saturates_and_the_extreme_corner_does_not_panic() {
    let long = Vector2::new(fixed(30_000), fixed(30_000));
    assert_eq!(
        long.length(),
        Fixed32::from_raw(i32::MAX),
        "about 42,426 units does not fit, so it saturates"
    );
    let corner = Vector2::new(Fixed32::from_raw(i32::MIN), Fixed32::from_raw(i32::MIN));
    assert_eq!(corner.length(), Fixed32::from_raw(i32::MAX));
    let direction = long.normalised();
    assert!(
        (direction.length().raw() - 65_536).abs() <= 2,
        "a long vector still normalises, got {direction:?}"
    );
    assert!(
        direction.x > Fixed32::ZERO && direction.y > Fixed32::ZERO,
        "and keeps its direction"
    );
}

#[test]
fn normalising_gives_length_one_within_rounding_and_zero_stays_zero() {
    for (x, y) in [(3, 4), (-7, 1), (1, -1), (100, 3)] {
        let length = Vector2::new(fixed(x), fixed(y)).normalised().length().raw();
        assert!(
            (length - 65_536).abs() <= 2,
            "({x}, {y}) normalised to {length}"
        );
    }
    assert_eq!(Vector2::ZERO.normalised(), Vector2::ZERO);
}

#[test]
fn dot_and_vector_operators_behave() {
    let a = Vector2::new(fixed(1), fixed(2));
    let b = Vector2::new(fixed(3), fixed(4));
    assert_eq!(a.dot(b), fixed(11));
    assert_eq!(a + b, Vector2::new(fixed(4), fixed(6)));
    assert_eq!(b - a, Vector2::new(fixed(2), fixed(2)));
    assert_eq!(-a, Vector2::new(fixed(-1), fixed(-2)));
    assert_eq!(a * fixed(2), Vector2::new(fixed(2), fixed(4)));
    assert_eq!(a.distance(b), Vector2::new(fixed(2), fixed(2)).length());
}

/// Walks from `start` to `target` and checks every property of every step.
fn walk(start: Vector2, target: Vector2, step: Fixed32, limit: usize) {
    let mut position = start;
    let mut previous_distance = position.distance(target);
    for _ in 0..limit {
        let (next, arrived) = position.toward(target, step);
        assert!(
            position.distance(next) <= step,
            "moved {:?}, farther than the step {step:?}",
            position.distance(next)
        );
        let distance = next.distance(target);
        assert!(
            distance <= previous_distance,
            "the distance grew from {previous_distance:?} to {distance:?}"
        );
        if arrived {
            assert_eq!(next, target, "arrival is exact");
            return;
        }
        assert!(next != position, "a positive step makes progress");
        position = next;
        previous_distance = distance;
    }
    panic!("never arrived from {start:?} to {target:?} with step {step:?}");
}

#[test]
fn toward_never_overshoots_always_progresses_and_arrives_exactly() {
    let mut streams = Streams::new(11);
    for _ in 0..1_000 {
        let mut point = || {
            Vector2::new(
                Fixed32::from_raw(streams.range("toward", -500_000, 500_000)),
                Fixed32::from_raw(streams.range("toward", -500_000, 500_000)),
            )
        };
        let (start, target) = (point(), point());
        walk(
            start,
            target,
            Fixed32::from_raw(streams.range("toward", 20_000, 200_000)),
            1_000,
        );
    }
}

#[test]
fn toward_arrives_even_with_a_one_raw_unit_step_along_a_diagonal() {
    let target = Vector2::new(Fixed32::from_raw(100), Fixed32::from_raw(100));
    walk(Vector2::ZERO, target, Fixed32::from_raw(1), 1_000);
    walk(
        target,
        Vector2::new(Fixed32::from_raw(-60), Fixed32::from_raw(37)),
        Fixed32::from_raw(1),
        1_000,
    );
}

#[test]
fn toward_works_over_distances_longer_than_fixed_point_can_hold_as_a_length() {
    let far = Vector2::new(fixed(30_000), fixed(30_000));
    let (next, arrived) = Vector2::ZERO.toward(far, Fixed32::from_raw(1));
    assert!(
        !arrived,
        "a step of one raw unit does not reach a target 42,000 units away"
    );
    assert!(next.length() <= Fixed32::from_raw(1));
    let (next, arrived) = Vector2::ZERO.toward(far, fixed(100));
    assert!(!arrived);
    assert!(next.length() <= fixed(100) && next.length() >= fixed(99));
}

/// The exact distance between two points in raw units, computed here in 64-bit floats so the test does
/// not share the code under test.
fn true_distance(a: Vector2, b: Vector2) -> f64 {
    let dx = a.x.raw() as f64 - b.x.raw() as f64;
    let dy = a.y.raw() as f64 - b.y.raw() as f64;
    (dx * dx + dy * dy).sqrt()
}

#[test]
fn a_walk_across_the_whole_range_heads_straight_for_the_target() {
    // Twenty thousand units either side of zero: the difference of the two is 40,000, which does not
    // fit in 16.16 and would wrap to the wrong side if it were subtracted in fixed point.
    let start = Vector2::new(fixed(-20_000), fixed(0));
    let target = Vector2::new(fixed(20_000), fixed(0));
    let step = fixed(500);
    let mut position = start;
    let mut previous = true_distance(position, target);
    for count in 1..=80 {
        let (next, arrived) = position.toward(target, step);
        assert!(
            next.x > position.x,
            "step {count} moved the wrong way: {position:?} to {next:?}"
        );
        assert_eq!(next.y, Fixed32::ZERO);
        let now = true_distance(next, target);
        assert!(
            now < previous,
            "step {count}: the true distance did not shrink"
        );
        assert!(
            previous - now <= step.raw() as f64 + 1.0,
            "step {count} moved farther than the step"
        );
        previous = now;
        position = next;
        if arrived {
            assert_eq!(
                (count, position),
                (80, target),
                "40,000 units at 500 a step is 80 steps"
            );
            return;
        }
    }
    panic!("never arrived");
}

#[test]
fn a_diagonal_walk_across_the_whole_range_arrives_and_never_gets_farther() {
    let start = Vector2::new(fixed(-30_000), fixed(25_000));
    let target = Vector2::new(fixed(30_000), fixed(-25_000));
    let mut position = start;
    let mut previous = true_distance(position, target);
    for _ in 0..2_000 {
        let (next, arrived) = position.toward(target, fixed(100));
        let now = true_distance(next, target);
        assert!(now <= previous, "the true distance grew");
        (position, previous) = (next, now);
        if arrived {
            assert_eq!(position, target);
            return;
        }
    }
    panic!("never arrived");
}

#[test]
fn distance_is_exact_across_the_whole_range_and_saturates_when_it_cannot_fit() {
    let left = Vector2::new(fixed(-20_000), fixed(0));
    let right = Vector2::new(fixed(20_000), fixed(0));
    assert_eq!(
        left.distance(right),
        Fixed32::from_raw(i32::MAX),
        "40,000 units does not fit, so it saturates"
    );
    let near = Vector2::new(fixed(-10_000), fixed(0));
    assert_eq!(
        near.distance(Vector2::new(fixed(10_000), fixed(0))),
        fixed(20_000)
    );
    assert_eq!(left.distance(left), Fixed32::ZERO);
}

#[test]
fn round_does_not_wrap_at_the_top_of_the_range() {
    assert_eq!(Fixed32::from_raw(i32::MAX).round(), 32_768);
    assert_eq!(
        Fixed32::from_raw(i32::MAX - 32_768).round(),
        32_767,
        "just under the tie"
    );
    assert_eq!(
        Fixed32::from_raw(i32::MAX - 32_767).round(),
        32_768,
        "exactly the tie rounds up"
    );
    assert_eq!(Fixed32::from_raw(i32::MIN).round(), -32_768);
    assert_eq!(
        Fixed32::from_raw(3 * 65_536 + 32_768).round(),
        4,
        "ties round up"
    );
    assert_eq!(
        Fixed32::from_raw(-3 * 65_536 - 32_768).round(),
        -3,
        "ties round up, so minus three and a half is minus three"
    );
}

#[test]
fn division_and_ratios_that_do_not_fit_wrap_and_do_not_panic() {
    assert_eq!(
        fixed(1_000) / Fixed32::from_raw(1),
        Fixed32::from_raw((1_000i64 << 32) as i32)
    );
    assert_eq!(
        Fixed32::from_ratio(40_000, 1),
        Fixed32::from_raw((40_000i64 << 16) as i32)
    );
}

#[test]
fn a_negative_or_zero_step_never_moves_and_never_panics() {
    let target = Vector2::new(fixed(3), fixed(4));
    assert_eq!(
        Vector2::ZERO.toward(target, fixed(-2)),
        (Vector2::ZERO, false)
    );
    assert_eq!(
        Vector2::ZERO.toward(target, Fixed32::ZERO),
        (Vector2::ZERO, false)
    );
    assert_eq!(
        target.toward(target, fixed(-2)),
        (target, true),
        "already there"
    );
}

#[test]
fn toward_arrives_at_once_when_the_target_is_within_one_step() {
    let origin = Vector2::ZERO;
    let target = Vector2::new(fixed(3), fixed(4));
    assert_eq!(origin.toward(target, fixed(5)), (target, true));
    assert_eq!(origin.toward(target, fixed(6)), (target, true));
    let (next, arrived) = origin.toward(target, fixed(1));
    assert!(!arrived);
    assert!(next.length() <= fixed(1));
}

// ------------------------------------------------------------------- angles

#[test]
fn the_generated_tables_equal_the_committed_fixtures() {
    let sine: Vec<u8> = generate_quarter_sine()
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect();
    assert_eq!(sine, include_bytes!("../../fixtures/quarter_sine.bin"));
    let atan: Vec<u8> = generate_atan_octant()
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect();
    assert_eq!(atan, include_bytes!("../../fixtures/atan_octant.bin"));
}

#[test]
fn the_cardinal_angles_are_exact() {
    assert_eq!(sin(0), Fixed32::ZERO);
    assert_eq!(sin(16_384), Fixed32::ONE);
    assert_eq!(sin(32_768), Fixed32::ZERO);
    assert_eq!(sin(49_152), -Fixed32::ONE);
    assert_eq!(cos(0), Fixed32::ONE);
    assert_eq!(cos(16_384), Fixed32::ZERO);
    assert_eq!(cos(32_768), -Fixed32::ONE);
    assert_eq!(
        sin(8_192).raw(),
        46_341,
        "sine of an eighth of a turn, 0.7071"
    );
}

#[test]
fn sine_squared_plus_cosine_squared_is_one_within_eight_raw_units_everywhere() {
    for turn in 0..=u16::MAX {
        let (s, c) = (sin(turn).raw() as i64, cos(turn).raw() as i64);
        let sum = (s * s + c * c) >> 16;
        assert!((sum - 65_536).abs() <= 8, "turn {turn}: {sum}");
    }
}

#[test]
fn sine_follows_the_true_curve_within_two_raw_units() {
    for turn in (0..=u16::MAX).step_by(7) {
        let radians = turn as f64 / 65_536.0 * std::f64::consts::TAU;
        let expected = (radians.sin() * 65_536.0).round() as i64;
        assert!(
            (sin(turn).raw() as i64 - expected).abs() <= 2,
            "turn {turn}"
        );
    }
}

#[test]
fn sine_is_odd_and_cosine_is_even() {
    for turn in 1..32_768u16 {
        assert_eq!(
            sin(turn.wrapping_neg()).raw(),
            -sin(turn).raw(),
            "turn {turn}"
        );
        assert_eq!(cos(turn.wrapping_neg()), cos(turn), "turn {turn}");
    }
}

fn turn_distance(a: u16, b: u16) -> u16 {
    let difference = a.wrapping_sub(b);
    difference.min(difference.wrapping_neg())
}

#[test]
fn arctangent_round_trips_every_angle_within_four_units() {
    for turn in 0..=u16::MAX {
        let found = atan2(sin(turn), cos(turn));
        assert!(
            turn_distance(found, turn) <= 4,
            "turn {turn} came back as {found}"
        );
    }
}

#[test]
fn arctangent_is_within_one_turn_unit_of_the_true_angle() {
    // 0.01 degrees is about 1.8 turn units.
    let mut worst = 0;
    for y in (-3_000..=3_000).step_by(37) {
        for x in (-3_000..=3_000).step_by(41) {
            if x == 0 && y == 0 {
                continue;
            }
            let expected =
                ((y as f64).atan2(x as f64) / std::f64::consts::TAU * 65_536.0).round() as i64;
            let found = atan2(Fixed32::from_raw(y * 20), Fixed32::from_raw(x * 20));
            worst = worst.max(turn_distance(found, expected as u16));
        }
    }
    assert!(worst <= 1, "worst error {worst} turn units");
}

#[test]
fn arctangent_handles_the_axes_and_the_origin() {
    assert_eq!(atan2(Fixed32::ZERO, Fixed32::ZERO), 0);
    assert_eq!(atan2(Fixed32::ZERO, fixed(5)), 0);
    assert_eq!(atan2(fixed(5), Fixed32::ZERO), 16_384);
    assert_eq!(atan2(Fixed32::ZERO, fixed(-5)), 32_768);
    assert_eq!(atan2(fixed(-5), Fixed32::ZERO), 49_152);
    assert_eq!(atan2(fixed(5), fixed(5)), 8_192);
    assert_eq!(
        atan2(Fixed32::from_raw(i32::MIN), Fixed32::from_raw(i32::MIN)),
        40_960
    );
}

#[test]
fn unit_vectors_have_length_one_within_rounding() {
    for turn in (0..=u16::MAX).step_by(13) {
        let length = unit(turn).length().raw();
        assert!((length - 65_536).abs() <= 8, "turn {turn}: {length}");
    }
}

#[test]
fn the_circle_table_is_symmetric_and_lands_on_the_radius() {
    assert_eq!(unit_circle_table(0, 8, 50), (50, 0));
    assert_eq!(unit_circle_table(2, 8, 50), (0, 50));
    assert_eq!(unit_circle_table(4, 8, 50), (-50, 0));
    assert_eq!(unit_circle_table(6, 8, 50), (0, -50));
    assert_eq!(unit_circle_table(1, 8, 50), (35, 35));
    for index in 0..360 {
        let (x, y) = unit_circle_table(index, 360, 50);
        let radius = ((x * x + y * y) as f64).sqrt();
        assert!(
            (radius - 50.0).abs() <= 0.8,
            "point {index} is at radius {radius}"
        );
    }
}

// ------------------------------------------------------- committed hash

fn math_hash() -> u64 {
    let square_roots: Vec<i32> = sqrt_samples()
        .into_iter()
        .map(|raw| Fixed32::from_raw(raw).sqrt().raw())
        .collect();
    let sines: Vec<i32> = (0..=u16::MAX).map(|turn| sin(turn).raw()).collect();
    let mut angles = Vec::new();
    for y in (-3_000..=3_000).step_by(37) {
        for x in (-3_000..=3_000).step_by(41) {
            angles.push(atan2(Fixed32::from_raw(y * 20), Fixed32::from_raw(x * 20)));
        }
    }
    hash_of(&(square_roots, sines, angles))
}

#[test]
fn the_math_results_hash_to_the_committed_value_natively_and_under_webassembly() {
    let committed = include_str!("../../fixtures/math_samples.hash").trim();
    assert_eq!(format!("{:016x}", math_hash()), committed);
}

/// Writes the committed fixtures. Run with
/// `cargo test -p lockstep-core regenerate_math_fixtures -- --ignored`, then commit the files with
/// a message that names the rule that changed.
#[test]
#[cfg(not(target_arch = "wasm32"))]
#[ignore]
fn regenerate_math_fixtures() {
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
    let sine: Vec<u8> = generate_quarter_sine()
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect();
    std::fs::write(format!("{directory}/quarter_sine.bin"), sine).unwrap();
    let atan: Vec<u8> = generate_atan_octant()
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect();
    std::fs::write(format!("{directory}/atan_octant.bin"), atan).unwrap();
    std::fs::write(
        format!("{directory}/math_samples.hash"),
        format!("{:016x}\n", math_hash()),
    )
    .unwrap();
}

#[test]
fn decimals_parse_exactly_and_round_ties_away_from_zero() {
    use lockstep_core::math::ParseFixedError;
    let parse = |text: &str| Fixed32::parse_decimal(text);
    assert_eq!(parse("1.5"), Ok(Fixed32::from_ratio(3, 2)));
    assert_eq!(parse("-12.375"), Ok(Fixed32::from_ratio(-99, 8)));
    assert_eq!(parse("+7"), Ok(Fixed32::from_int(7)));
    assert_eq!(parse(".25"), Ok(Fixed32::from_ratio(1, 4)));
    assert_eq!(parse("3."), Ok(Fixed32::from_int(3)));
    assert_eq!(
        parse("0.0000152587890625"),
        Ok(Fixed32::from_raw(1)),
        "2^-16 exactly"
    );
    // Half a raw unit rounds away from zero, a hair less rounds toward it.
    assert_eq!(parse("0.00000762939453125"), Ok(Fixed32::from_raw(1)));
    assert_eq!(parse("-0.00000762939453125"), Ok(Fixed32::from_raw(-1)));
    assert_eq!(parse("0.000007629394531"), Ok(Fixed32::ZERO));
    // 0.1 is not exact in binary: the nearest raw unit is 6,554.
    assert_eq!(parse("0.1"), Ok(Fixed32::from_raw(6_554)));
    assert_eq!("2.25".parse::<Fixed32>(), Ok(Fixed32::from_ratio(9, 4)));
    assert_eq!(parse("-32768"), Ok(Fixed32::from_raw(i32::MIN)));
    assert_eq!(parse("32768"), Err(ParseFixedError::OutOfRange));
    // Rounding happens before the range check: just under the top rounds out of range, and the
    // same value negated rounds to exactly the bottom.
    assert_eq!(parse("32767.99999999"), Err(ParseFixedError::OutOfRange));
    assert_eq!(parse("-32767.99999999"), Ok(Fixed32::from_raw(i32::MIN)));
    assert_eq!(parse("-0"), Ok(Fixed32::ZERO));
    assert_eq!(parse("-0.0000001"), Ok(Fixed32::ZERO));
    assert_eq!(
        parse("99999999999999999999999"),
        Err(ParseFixedError::OutOfRange)
    );
    for bad in [
        "", "-", "+", ".", "1.2.3", "1e5", "0x10", " 1", "1,5", "--1",
    ] {
        assert_eq!(parse(bad), Err(ParseFixedError::NotADecimal), "{bad:?}");
    }
    assert_eq!(
        parse("0.1234567890123456789"),
        Err(ParseFixedError::TooManyDigits)
    );
}
