use super::*;

#[test]
fn trims_surrounding_whitespace() {
    assert_eq!(UserId::new("  u-1 ").unwrap().as_str(), "u-1");
}

#[test]
fn rejects_blank_value() {
    assert_eq!(UserId::new("   "), Err(DomainError::EmptyField("user_id")));
}

#[test]
fn rejects_value_over_max_len() {
    assert_eq!(
        ContentId::new("x".repeat(MAX_LEN + 1)),
        Err(DomainError::TooLong {
            field: "content_id",
            max: MAX_LEN
        })
    );
}
