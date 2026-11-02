use super::*;

fn parse(json: &str) -> RawEvent {
    let element: Box<RawValue> = serde_json::from_str(json).unwrap();
    raw_event(&element)
}

#[test]
fn reads_id_and_type_and_keeps_payload_verbatim() {
    let json = r#"{ "event_id": "e-1", "source": "user_behavior",
                    "event": { "type": "content_viewed" }, "extra": [1, 2] }"#;

    let event = parse(json);

    assert_eq!(event.payload, json.as_bytes());
    assert_eq!(event.event_id.as_deref(), Some("e-1"));
    assert_eq!(
        event.event_type.as_deref(),
        Some("user_behavior.content_viewed")
    );
}

#[test]
fn tolerates_missing_or_mistyped_fields() {
    let event = parse(r#"{ "event_id": 42, "source": "user_data" }"#);

    assert_eq!(event.event_id, None);
    assert_eq!(event.event_type, None);
}

#[test]
fn accepts_elements_that_are_not_objects() {
    let event = parse(r#""solo un texto""#);

    assert_eq!(event.payload, br#""solo un texto""#);
    assert_eq!(event.event_id, None);
}
