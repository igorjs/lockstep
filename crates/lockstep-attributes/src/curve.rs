use lockstep_core::math::Fixed32;

/// How a derived attribute turns its inputs into a value. Every curve saturates at the edges of
/// the 16.16 range instead of wrapping.
#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    /// `input × per_point + offset`: critical chance as 1 percent per point of Luck.
    Linear { per_point: Fixed32, offset: Fixed32 },
    /// Straight lines between knots sorted by input, held flat past the first and last knot.
    Piecewise { knots: Vec<(Fixed32, Fixed32)> },
    /// A step function: the value of the last step at or below the input, and the first step's
    /// value below every step.
    Threshold { steps: Vec<(Fixed32, Fixed32)> },
    /// Every input multiplied together.
    Product,
    /// Every input added together.
    Sum,
    /// The first input minus all the others.
    Difference,
}

impl Curve {
    /// How many inputs the curve reads: exactly one, or one or more.
    pub(crate) fn takes_one_input(&self) -> bool {
        matches!(
            self,
            Curve::Linear { .. } | Curve::Piecewise { .. } | Curve::Threshold { .. }
        )
    }

    /// The curve's value for these inputs. Pass as many inputs as the curve reads: a registry checks
    /// that for derived attributes.
    pub fn evaluate(&self, inputs: &[Fixed32]) -> Fixed32 {
        let raw = |value: Fixed32| value.raw() as i128;
        match self {
            Curve::Linear { per_point, offset } => {
                saturate(((raw(inputs[0]) * raw(*per_point)) >> 16) + raw(*offset))
            }
            Curve::Piecewise { knots } => {
                let x = inputs[0];
                let (first, last) = (knots[0], knots[knots.len() - 1]);
                if x <= first.0 {
                    return first.1;
                }
                if x >= last.0 {
                    return last.1;
                }
                let index = knots
                    .iter()
                    .position(|knot| knot.0 > x)
                    .expect("x is below the last knot");
                let ((x0, y0), (x1, y1)) = (knots[index - 1], knots[index]);
                let rise = (raw(y1) - raw(y0)) * (raw(x) - raw(x0));
                saturate(raw(y0) + divide_rounded(rise, raw(x1) - raw(x0)))
            }
            Curve::Threshold { steps } => {
                steps
                    .iter()
                    .rev()
                    .find(|step| step.0 <= inputs[0])
                    .unwrap_or(&steps[0])
                    .1
            }
            Curve::Product => {
                // The running product is held to 2^90 raw, which the next multiply by a 32-bit
                // input cannot overflow, and saturates once at the end. Clamping to 16.16 after
                // every input would make the answer depend on input order.
                let bound = 1i128 << 90;
                let mut product = raw(Fixed32::ONE);
                for input in inputs {
                    product = ((product * raw(*input)) >> 16).clamp(-bound, bound);
                }
                saturate(product)
            }
            Curve::Sum => saturate(inputs.iter().map(|input| raw(*input)).sum()),
            Curve::Difference => {
                saturate(raw(inputs[0]) - inputs[1..].iter().map(|input| raw(*input)).sum::<i128>())
            }
        }
    }
}

/// Nearest, with ties away from zero. The divisor is positive.
pub(crate) fn divide_rounded(numerator: i128, divisor: i128) -> i128 {
    let half = divisor / 2;
    if numerator >= 0 {
        (numerator + half) / divisor
    } else {
        -((-numerator + half) / divisor)
    }
}

pub(crate) fn saturate(raw: i128) -> Fixed32 {
    Fixed32::from_raw(raw.clamp(i32::MIN as i128, i32::MAX as i128) as i32)
}
