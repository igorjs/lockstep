use crate::common::whole;
use lockstep_attributes::Curve;
use lockstep_core::math::Fixed32;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn knots(points: &[(i32, i32)]) -> Vec<(Fixed32, Fixed32)> {
    points.iter().map(|(x, y)| (whole(*x), whole(*y))).collect()
}

#[test]
fn linear_multiplies_and_offsets_and_saturates_instead_of_wrapping() {
    let curve = Curve::Linear {
        per_point: Fixed32::ONE,
        offset: whole(5),
    };
    assert_eq!(curve.evaluate(&[whole(3)]), whole(8));
    let steep = Curve::Linear {
        per_point: whole(1_000),
        offset: whole(0),
    };
    assert_eq!(steep.evaluate(&[whole(1_000)]), Fixed32::from_raw(i32::MAX));
    assert_eq!(
        steep.evaluate(&[whole(-1_000)]),
        Fixed32::from_raw(i32::MIN)
    );
}

#[test]
fn piecewise_interpolates_between_knots_and_holds_past_the_ends() {
    let curve = Curve::Piecewise {
        knots: knots(&[(0, 10), (10, 30), (20, 30)]),
    };
    for (input, expected) in [(-5, 10), (0, 10), (5, 20), (10, 30), (15, 30), (50, 30)] {
        assert_eq!(curve.evaluate(&[whole(input)]), whole(expected), "{input}");
    }
    // A third is 21,845 raw units; two raw units of output per unit of input gives 43,690.
    assert_eq!(
        curve.evaluate(&[Fixed32::from_ratio(1, 3)]),
        whole(10) + Fixed32::from_raw(43_690)
    );
}

#[test]
fn threshold_steps_take_the_last_step_at_or_below_the_input() {
    let curve = Curve::Threshold {
        steps: knots(&[(0, 1), (10, 2), (20, 3)]),
    };
    for (input, expected) in [(-5, 1), (0, 1), (9, 1), (10, 2), (19, 2), (20, 3), (99, 3)] {
        assert_eq!(curve.evaluate(&[whole(input)]), whole(expected), "{input}");
    }
}

#[test]
fn product_sum_and_difference_combine_every_input() {
    let inputs = [whole(6), Fixed32::from_ratio(3, 2), whole(-2)];
    assert_eq!(Curve::Product.evaluate(&inputs), whole(-18));
    assert_eq!(Curve::Sum.evaluate(&inputs), Fixed32::from_ratio(11, 2));
    assert_eq!(
        Curve::Difference.evaluate(&inputs),
        Fixed32::from_ratio(13, 2)
    );
    assert_eq!(Curve::Difference.evaluate(&[whole(4)]), whole(4));
}
