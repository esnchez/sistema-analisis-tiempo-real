use super::*;
use crate::domain::{
    ContentId, ContentRef, ContentType, CountryCode, Currency, DeviceType, Money, Platform,
    SessionId, TrafficSource,
};

fn web_context() -> ClientContext {
    ClientContext {
        session_id: Some(SessionId::new("s-981").unwrap()),
        authenticated: true,
        platform: Platform::Web,
        device_type: DeviceType::Mobile,
        app_version: None,
        country: Some(CountryCode::new("ES").unwrap()),
    }
}

#[test]
fn deserializes_user_behavior_event_with_context() {
    let json = r#"{
        "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
        "user_id": "u-42",
        "occurred_at": "2026-10-06T10:15:00Z",
        "context": {
            "session_id": "s-981",
            "authenticated": true,
            "platform": "web",
            "device_type": "mobile",
            "country": "es"
        },
        "source": "user_behavior",
        "event": {
            "type": "content_viewed",
            "content": { "id": "article-123", "content_type": "article" },
            "traffic_source": "social"
        }
    }"#;

    let event: SourceEvent = serde_json::from_str(json).unwrap();

    assert_eq!(event.event_type(), "user_behavior.content_viewed");
    assert_eq!(event.context(), Some(&web_context()));
    assert_eq!(
        event.payload(),
        &EventPayload::UserBehavior(UserBehaviorEvent::ContentViewed {
            content: ContentRef::new(ContentId::new("article-123").unwrap(), ContentType::Article),
            traffic_source: TrafficSource::Social,
            referrer: None,
            campaign: None,
        })
    );
}

#[test]
fn deserializes_user_data_event_without_context() {
    let json = r#"{
        "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
        "user_id": "u-42",
        "occurred_at": "2026-10-06T10:15:00Z",
        "source": "user_data",
        "event": {
            "type": "subscription_changed",
            "change": "upgraded",
            "plan": "premium",
            "previous_plan": "basic",
            "billing_period": "yearly",
            "price": { "amount_minor": 9900, "currency": "EUR" }
        }
    }"#;

    let event: SourceEvent = serde_json::from_str(json).unwrap();

    assert_eq!(event.context(), None);
    assert_eq!(
        event.payload(),
        &EventPayload::UserData(UserDataEvent::SubscriptionChanged {
            change: SubscriptionChange::Upgraded,
            plan: SubscriptionPlan::Premium,
            previous_plan: Some(SubscriptionPlan::Basic),
            billing_period: Some(BillingPeriod::Yearly),
            price: Some(Money {
                amount_minor: 9900,
                currency: Currency::new("EUR").unwrap(),
            }),
            trial: false,
        })
    );
}

#[test]
fn deserializes_unit_variant() {
    let json = r#"{
        "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
        "user_id": "u-42",
        "occurred_at": "2026-10-06T10:15:00Z",
        "source": "user_data",
        "event": { "type": "account_deleted" }
    }"#;

    let event: SourceEvent = serde_json::from_str(json).unwrap();

    assert_eq!(event.event_type(), "user_data.account_deleted");
}

#[test]
fn roundtrips_through_json() {
    let event = SourceEvent::new(
        UserId::new("u-42").unwrap(),
        Utc::now(),
        Some(web_context()),
        EventPayload::UserBehavior(UserBehaviorEvent::PlaybackStateChanged {
            content: ContentRef::new(ContentId::new("video-7").unwrap(), ContentType::Video),
            action: PlaybackAction::Paused,
            position_secs: 93,
            duration_secs: Some(240),
            bitrate_kbps: Some(4500),
        }),
    );

    let json = serde_json::to_string(&event).unwrap();

    assert_eq!(serde_json::from_str::<SourceEvent>(&json).unwrap(), event);
}

#[test]
fn rejects_blank_user_id() {
    let json = r#"{
        "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
        "user_id": " ",
        "occurred_at": "2026-10-06T10:15:00Z",
        "source": "user_data",
        "event": { "type": "account_deleted" }
    }"#;

    assert!(serde_json::from_str::<SourceEvent>(json).is_err());
}

#[test]
fn rejects_scroll_depth_over_100() {
    let json = r#"{
        "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
        "user_id": "u-42",
        "occurred_at": "2026-10-06T10:15:00Z",
        "source": "user_behavior",
        "event": {
            "type": "content_engaged",
            "content": { "id": "article-123", "content_type": "article" },
            "engaged_secs": 45,
            "scroll_depth": 120
        }
    }"#;

    assert!(serde_json::from_str::<SourceEvent>(json).is_err());
}

#[test]
fn rejects_unknown_event_type() {
    let json = r#"{
        "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
        "user_id": "u-42",
        "occurred_at": "2026-10-06T10:15:00Z",
        "source": "user_behavior",
        "event": { "type": "teleported" }
    }"#;

    assert!(serde_json::from_str::<SourceEvent>(json).is_err());
}
