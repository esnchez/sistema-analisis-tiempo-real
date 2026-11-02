use chrono::TimeZone;

use super::*;
use crate::domain::event::{
    AdAction, AdPlacement, AgeRange, PlaybackAction, SignupMethod, SubscriptionChange,
    SubscriptionPlan,
};
use crate::domain::{
    ContentId, ContentType, Currency, DeviceType, Platform, SessionId, TrafficSource, UserId,
};

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 6, hour, 0, 0).unwrap()
}

fn event(context: Option<ClientContext>, payload: EventPayload) -> SourceEvent {
    SourceEvent::new(UserId::new("u-42").unwrap(), at(10), context, payload)
}

fn video() -> ContentRef {
    ContentRef {
        section: Some("deportes".into()),
        published_at: Some(at(8)),
        paywalled: true,
        ..ContentRef::new(ContentId::new("video-7").unwrap(), ContentType::LiveStream)
    }
}

#[test]
fn flattens_identity_context_and_content() {
    let context = ClientContext {
        session_id: Some(SessionId::new("s-981").unwrap()),
        authenticated: true,
        platform: Platform::SmartTv,
        device_type: DeviceType::Tv,
        app_version: Some("3.2.0".into()),
        country: Some(CountryCode::new("ES").unwrap()),
    };
    let event = event(
        Some(context),
        EventPayload::UserBehavior(UserBehaviorEvent::PlaybackStateChanged {
            content: video(),
            action: PlaybackAction::BufferingStarted,
            position_secs: 93,
            duration_secs: None,
            bitrate_kbps: Some(4500),
        }),
    );

    let row = EventRow::from_event(&event, at(11));

    assert_eq!(
        row,
        EventRow {
            event_id: *event.event_id().as_uuid(),
            event_type: "user_behavior.playback_state_changed".into(),
            user_id: "u-42".into(),
            occurred_at: at(10),
            ingested_at: at(11),
            session_id: "s-981".into(),
            authenticated: true,
            platform: "smart_tv".into(),
            device_type: "tv".into(),
            app_version: "3.2.0".into(),
            country: "ES".into(),
            content_id: "video-7".into(),
            content_type: "live_stream".into(),
            content_section: "deportes".into(),
            content_published_at: Some(at(8)),
            content_paywalled: true,
            playback_action: "buffering_started".into(),
            position_secs: 93,
            bitrate_kbps: Some(4500),
            ..EventRow::default()
        }
    );
}

#[test]
fn flattens_subscription_change_without_context() {
    let event = event(
        None,
        EventPayload::UserData(UserDataEvent::SubscriptionChanged {
            change: SubscriptionChange::Upgraded,
            plan: SubscriptionPlan::Premium,
            previous_plan: Some(SubscriptionPlan::Basic),
            billing_period: None,
            price: Some(Money {
                amount_minor: 9900,
                currency: Currency::new("EUR").unwrap(),
            }),
            trial: true,
        }),
    );

    let row = EventRow::from_event(&event, at(11));

    assert_eq!(row.session_id, "");
    assert_eq!(row.platform, "");
    assert_eq!(row.subscription_change, "upgraded");
    assert_eq!(row.subscription_plan, "premium");
    assert_eq!(row.previous_subscription_plan, "basic");
    assert_eq!(row.billing_period, "");
    assert_eq!(row.price_amount_minor, Some(9900));
    assert_eq!(row.price_currency, "EUR");
    assert!(row.trial);
}

#[test]
fn keeps_declared_country_apart_from_ip_country() {
    let event = event(
        None,
        EventPayload::UserData(UserDataEvent::Registered {
            signup_method: SignupMethod::Google,
            country: Some(CountryCode::new("MX").unwrap()),
            language: Some("es-MX".into()),
            age_range: Some(AgeRange::From25To34),
            traffic_source: Some(TrafficSource::Newsletter),
        }),
    );

    let row = EventRow::from_event(&event, at(11));

    assert_eq!(row.country, "");
    assert_eq!(row.profile_country, "MX");
    assert_eq!(row.age_range, "25_34");
    assert_eq!(row.traffic_source, "newsletter");
}

#[test]
fn ad_interaction_keeps_content_id_without_content_metadata() {
    let event = event(
        None,
        EventPayload::UserBehavior(UserBehaviorEvent::AdInteracted {
            ad_id: "ad-1".into(),
            placement: AdPlacement::PreRoll,
            action: AdAction::Skipped,
            content_id: Some(ContentId::new("video-7").unwrap()),
        }),
    );

    let row = EventRow::from_event(&event, at(11));

    assert_eq!(row.content_id, "video-7");
    assert_eq!(row.content_type, "");
    assert_eq!(row.ad_placement, "pre_roll");
    assert_eq!(row.ad_action, "skipped");
}
