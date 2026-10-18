// SPDX-License-Identifier: Apache-2.0
use lockstep_core::math::{Currency, Money, MoneyError, MAXIMUM_EXPONENT};
use lockstep_core::{decode, encode};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

fn dollars() -> Currency {
    Currency::new("USD", 2).unwrap()
}

fn euros() -> Currency {
    Currency::new("EUR", 2).unwrap()
}

fn amount(minor: i64) -> Money {
    Money::from_minor(minor, dollars())
}

#[test]
fn a_currency_needs_three_capital_letters_and_a_small_exponent() {
    assert!(Currency::new("USD", 2).is_ok());
    for bad in ["US", "USDX", "usd", "U5D", ""] {
        assert_eq!(Currency::new(bad, 2), Err(MoneyError::InvalidCurrency));
    }
    assert!(Currency::new("USD", MAXIMUM_EXPONENT).is_ok());
    assert_eq!(
        Currency::new("USD", MAXIMUM_EXPONENT + 1),
        Err(MoneyError::InvalidCurrency)
    );
}

#[test]
fn a_decoded_currency_is_checked_like_a_new_one() {
    // The exponent 200 would overflow ten to the power when displayed.
    let mut bytes = encode(&dollars());
    *bytes.last_mut().unwrap() = 200;
    assert!(decode::<Currency>(&bytes).is_err());
    assert_eq!(decode::<Currency>(&encode(&dollars())), Ok(dollars()));
}

#[test]
fn adding_and_subtracting_are_checked() {
    assert_eq!(amount(5).checked_add(amount(7)), Ok(amount(12)));
    assert_eq!(amount(5).checked_sub(amount(7)), Ok(amount(-2)));
    assert_eq!(
        amount(i64::MAX).checked_add(amount(1)),
        Err(MoneyError::Overflow)
    );
    assert_eq!(
        amount(i64::MIN).checked_sub(amount(1)),
        Err(MoneyError::Overflow)
    );
    assert_eq!(amount(i64::MIN).checked_neg(), Err(MoneyError::Overflow));
    assert_eq!(amount(9).checked_neg(), Ok(amount(-9)));
}

#[test]
fn different_currencies_never_mix() {
    let other = Money::from_minor(5, euros());
    assert_eq!(
        amount(5).checked_add(other),
        Err(MoneyError::CurrencyMismatch)
    );
    assert_eq!(
        amount(5).checked_sub(other),
        Err(MoneyError::CurrencyMismatch)
    );
}

#[test]
fn multiplying_is_checked() {
    assert_eq!(amount(12).checked_mul(-3), Ok(amount(-36)));
    assert_eq!(amount(i64::MAX).checked_mul(2), Err(MoneyError::Overflow));
    assert_eq!(amount(i64::MIN).checked_mul(-1), Err(MoneyError::Overflow));
}

#[test]
fn major_units_use_the_currency_exponent() {
    assert_eq!(Money::from_major(12, dollars()), Ok(amount(1_200)));
    let yen = Currency::new("JPY", 0).unwrap();
    assert_eq!(Money::from_major(12, yen), Ok(Money::from_minor(12, yen)));
    assert_eq!(
        Money::from_major(i64::MAX, dollars()),
        Err(MoneyError::Overflow)
    );
}

#[test]
fn a_sum_adds_every_amount_or_fails() {
    let parts = [amount(1), amount(2), amount(3)];
    assert_eq!(Money::checked_sum(parts, dollars()), Ok(amount(6)));
    assert_eq!(Money::checked_sum([], dollars()), Ok(amount(0)));
    assert_eq!(
        Money::checked_sum([amount(i64::MAX), amount(1)], dollars()),
        Err(MoneyError::Overflow)
    );
    assert_eq!(
        Money::checked_sum([Money::from_minor(1, euros())], dollars()),
        Err(MoneyError::CurrencyMismatch)
    );
}

#[test]
fn a_split_gives_the_extra_units_to_the_first_parts() {
    assert_eq!(
        amount(100).split(3).unwrap(),
        vec![amount(34), amount(33), amount(33)]
    );
    assert_eq!(
        amount(-100).split(3).unwrap(),
        vec![amount(-34), amount(-33), amount(-33)]
    );
    assert_eq!(amount(2).split(5).unwrap().len(), 5);
    assert_eq!(amount(5).split(0), Err(MoneyError::NothingToShareWith));
}

#[test]
fn an_allocation_follows_the_weights_and_zero_weights_get_nothing() {
    assert_eq!(
        amount(100).allocate(&[1, 0, 1]).unwrap(),
        vec![amount(50), amount(0), amount(50)]
    );
    assert_eq!(
        amount(10).allocate(&[3, 1]).unwrap(),
        vec![amount(8), amount(2)]
    );
    assert_eq!(
        amount(10).allocate(&[0, 0]),
        Err(MoneyError::NothingToShareWith)
    );
    assert_eq!(
        amount(10).allocate(&[]),
        Err(MoneyError::NothingToShareWith)
    );
}

#[test]
fn the_extremes_split_without_overflow() {
    for total in [i64::MIN, i64::MAX, 0, 1, -1] {
        for weights in [vec![u64::MAX, u64::MAX, 1], vec![1; 7], vec![u64::MAX]] {
            let parts = amount(total).allocate(&weights).unwrap();
            let sum: i128 = parts.iter().map(|part| part.minor() as i128).sum();
            assert_eq!(sum, total as i128);
        }
    }
}

#[test]
fn display_shows_major_units_with_the_currency_digits() {
    assert_eq!(amount(-1_205).to_string(), "-12.05 USD");
    assert_eq!(amount(5).to_string(), "0.05 USD");
    assert_eq!(
        Money::from_minor(7, Currency::new("JPY", 0).unwrap()).to_string(),
        "7 JPY"
    );
    assert_eq!(amount(i64::MIN).to_string(), "-92233720368547758.08 USD");
    let deep = Currency::new("WEI", MAXIMUM_EXPONENT).unwrap();
    assert_eq!(
        Money::from_minor(i64::MIN, deep).to_string(),
        "-9.223372036854775808 WEI"
    );
}
