use super::*;

#[test]
fn country_code_is_normalized_to_uppercase() {
    assert_eq!(CountryCode::new(" es ").unwrap().as_str(), "ES");
}

#[test]
fn country_code_rejects_wrong_length_or_non_letters() {
    assert!(CountryCode::new("ESP").is_err());
    assert!(CountryCode::new("E1").is_err());
}

#[test]
fn currency_requires_three_letters() {
    assert_eq!(Currency::new("eur").unwrap().as_str(), "EUR");
    assert!(Currency::new("EU").is_err());
}

#[test]
fn percent_accepts_bounds_and_rejects_above_100() {
    assert_eq!(Percent::new(0).unwrap().value(), 0);
    assert_eq!(Percent::new(100).unwrap().value(), 100);
    assert!(Percent::new(101).is_err());
}
