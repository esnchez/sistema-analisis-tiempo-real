//! Repositorio de eventos contra un ClickHouse real.
//!
//! El cliente valida cada columna de la fila contra el esquema de la tabla al insertar,
//! así que estas pruebas detectan cualquier desajuste entre `EventRow` y `config/clickhouse.sql`.

use chrono::Utc;
use data_processor::application::port::EventRepository;
use data_processor::domain::event::{
    EventPayload, PlaybackAction, SourceEvent, SubscriptionChange, SubscriptionPlan,
    UserBehaviorEvent, UserDataEvent,
};
use data_processor::domain::{
    ClientContext, ContentId, ContentRef, ContentType, CountryCode, Currency, DeviceType, Money,
    Platform, SessionId, TrafficSource, UserId,
};
use data_processor::infrastructure::clickhouse::{ClickHouseConfig, ClickHouseEventRepository};
use uuid::Uuid;

/// Usuario único por ejecución para no contar filas de ejecuciones anteriores.
fn unique_user() -> UserId {
    UserId::new(format!("it-{}", Uuid::now_v7())).unwrap()
}

/// Un evento de cada forma relevante: con contexto y contenido, con nulos y sin contexto.
fn sample_events(user: &UserId) -> Vec<SourceEvent> {
    let context = ClientContext {
        session_id: Some(SessionId::new("s-1").unwrap()),
        authenticated: true,
        platform: Platform::Web,
        device_type: DeviceType::Mobile,
        app_version: None,
        country: Some(CountryCode::new("ES").unwrap()),
    };
    let video = ContentRef {
        section: Some("deportes".into()),
        published_at: Some(Utc::now()),
        ..ContentRef::new(ContentId::new("video-7").unwrap(), ContentType::Video)
    };
    vec![
        SourceEvent::new(
            user.clone(),
            Utc::now(),
            Some(context.clone()),
            EventPayload::UserBehavior(UserBehaviorEvent::ContentViewed {
                content: video.clone(),
                traffic_source: TrafficSource::Social,
                referrer: None,
                campaign: Some("otoño".into()),
            }),
        ),
        SourceEvent::new(
            user.clone(),
            Utc::now(),
            Some(context),
            EventPayload::UserBehavior(UserBehaviorEvent::PlaybackStateChanged {
                content: video,
                action: PlaybackAction::Heartbeat,
                position_secs: 30,
                duration_secs: None,
                bitrate_kbps: Some(4500),
            }),
        ),
        SourceEvent::new(
            user.clone(),
            Utc::now(),
            None,
            EventPayload::UserData(UserDataEvent::SubscriptionChanged {
                change: SubscriptionChange::Started,
                plan: SubscriptionPlan::Premium,
                previous_plan: None,
                billing_period: None,
                price: Some(Money {
                    amount_minor: 999,
                    currency: Currency::new("EUR").unwrap(),
                }),
                trial: false,
            }),
        ),
    ]
}

/// Filas del usuario tras deduplicar en la lectura (`FINAL`).
async fn count_final(config: &ClickHouseConfig, user: &UserId) -> u64 {
    count(config, user, "FINAL").await
}

/// Filas físicas del usuario, sin deduplicar en la lectura.
async fn count_stored(config: &ClickHouseConfig, user: &UserId) -> u64 {
    count(config, user, "").await
}

async fn count(config: &ClickHouseConfig, user: &UserId, modifier: &str) -> u64 {
    config
        .client()
        .query(&format!(
            "SELECT count() FROM events {modifier} WHERE user_id = ?"
        ))
        .bind(user.as_str())
        .fetch_one()
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requiere ClickHouse: docker compose up -d clickhouse"]
async fn appends_events_matching_the_schema() {
    let config = ClickHouseConfig::from_env();
    let repository = ClickHouseEventRepository::new(config.client());
    let user = unique_user();
    let events = sample_events(&user);

    repository.append(&events).await.unwrap();

    assert_eq!(count_final(&config, &user).await, events.len() as u64);
}

#[tokio::test]
#[ignore = "requiere ClickHouse: docker compose up -d clickhouse"]
async fn retrying_a_batch_does_not_store_it_twice() {
    let config = ClickHouseConfig::from_env();
    let repository = ClickHouseEventRepository::new(config.client());
    let user = unique_user();
    let events = sample_events(&user);

    repository.append(&events).await.unwrap();
    repository.append(&events).await.unwrap();

    // Sin `FINAL`: el reintento se descartó al insertar (`insert_deduplication_token`),
    // no hizo falta esperar a la fusión de `ReplacingMergeTree`.
    assert_eq!(count_stored(&config, &user).await, events.len() as u64);
}
