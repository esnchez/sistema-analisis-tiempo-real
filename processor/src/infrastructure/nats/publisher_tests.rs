use super::*;

#[test]
fn routes_by_declared_type() {
    assert_eq!(
        subject("events", Some("user_behavior.content_viewed")),
        "events.user_behavior.content_viewed"
    );
}

#[test]
fn unusable_types_go_to_unknown() {
    for event_type in [
        None,
        Some(""),
        Some("no_dot"),
        Some("too.many.dots"),
        Some("user_behavior.*"),
        Some("user_behavior.>"),
        Some("User_Behavior.Viewed"),
        Some("user behavior.viewed"),
    ] {
        assert_eq!(
            subject("events", event_type),
            "events.unknown",
            "{event_type:?}"
        );
    }
}

#[test]
fn msg_id_must_be_a_single_line_of_bounded_length() {
    assert!(is_valid_msg_id("0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70"));
    assert!(!is_valid_msg_id(""));
    assert!(!is_valid_msg_id("a\r\nb"));
    assert!(!is_valid_msg_id(&"x".repeat(MAX_MSG_ID_LEN + 1)));
}
