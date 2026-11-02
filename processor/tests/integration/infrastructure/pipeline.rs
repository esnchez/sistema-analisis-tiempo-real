//! Pipeline de extremo a extremo: NATS JetStream real → procesador → ClickHouse real.
//!
//! Cada prueba crea sus propios streams (nombres únicos) y los borra al terminar.

use std::sync::Arc;
use std::time::Duration;

use async_nats::HeaderMap;
use async_nats::jetstream::Context;
use chrono::Utc;
use data_processor::application::command::RecordEventsHandler;
use data_processor::domain::UserId;
use data_processor::domain::event::{EventPayload, SourceEvent, UserDataEvent};
use data_processor::infrastructure::clickhouse::{ClickHouseConfig, ClickHouseEventRepository};
use data_processor::infrastructure::nats::{self, NatsConfig, NatsDeadLetterQueue};
use data_processor::infrastructure::pipeline::{self, BatchLimits, PipelineConfig};
use tokio::time::{Instant, sleep};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Configuración con streams y subjects exclusivos de esta ejecución.
fn isolated_nats_config() -> NatsConfig {
    let id = Uuid::now_v7().simple().to_string();
    NatsConfig {
        stream: format!("IT_{id}"),
        subjects: vec![format!("it.{id}.>")],
        dlq_stream: format!("IT_DLQ_{id}"),
        dlq_subject_prefix: format!("it-dlq.{id}"),
        ..NatsConfig::from_env().unwrap()
    }
}

fn subject(config: &NatsConfig) -> String {
    config.subjects[0].replace('>', "user_data.account_deleted")
}

fn event(user: &UserId) -> SourceEvent {
    SourceEvent::new(
        user.clone(),
        Utc::now(),
        None,
        EventPayload::UserData(UserDataEvent::AccountDeleted),
    )
}

/// Publica con `Nats-Msg-Id`, como deben hacer los productores.
async fn publish(js: &Context, subject: &str, msg_id: &str, payload: Vec<u8>) {
    let mut headers = HeaderMap::new();
    headers.insert("Nats-Msg-Id", msg_id);
    js.publish_with_headers(subject.to_owned(), headers, payload.into())
        .await
        .unwrap()
        .await
        .unwrap();
}

async fn count_final(config: &ClickHouseConfig, user: &UserId) -> u64 {
    config
        .client()
        .query("SELECT count() FROM events FINAL WHERE user_id = ?")
        .bind(user.as_str())
        .fetch_one()
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requiere NATS y ClickHouse: docker compose up -d"]
async fn persists_valid_events_and_dead_letters_invalid_ones() {
    let nats_config = isolated_nats_config();
    let clickhouse: ClickHouseConfig = ClickHouseConfig::from_env();
    let js = nats::connect(&nats_config).await.unwrap();
    let consumer = {
        nats::ensure_streams(&js, &nats_config).await.unwrap();
        nats::ensure_consumer(&js, &nats_config).await.unwrap()
    };
    let subject = subject(&nats_config);
    let user = UserId::new(format!("it-{}", Uuid::now_v7())).unwrap();
    let events = [event(&user), event(&user), event(&user)];

    for event in &events {
        let id = event.event_id().to_string();
        publish(&js, &subject, &id, serde_json::to_vec(event).unwrap()).await;
    }
    // Reintento del productor: mismo `Nats-Msg-Id` → JetStream lo descarta.
    let first = &events[0];
    let first_id = first.event_id().to_string();
    publish(&js, &subject, &first_id, serde_json::to_vec(first).unwrap()).await;
    // Mensaje venenoso → DLQ.
    publish(&js, &subject, "poison-1", b"{ no es json".to_vec()).await;

    let source = nats::deliveries(&consumer, &nats_config).await.unwrap();
    let dlq = Arc::new(NatsDeadLetterQueue::new(js.clone(), &nats_config));
    let repository = ClickHouseEventRepository::new(clickhouse.client());
    let handler = Arc::new(RecordEventsHandler::new(repository));
    let shutdown = CancellationToken::new();
    let config = PipelineConfig {
        batch: BatchLimits {
            max_age: Duration::from_millis(200),
            ..BatchLimits::default()
        },
        ..PipelineConfig::default()
    };
    let running = tokio::spawn(pipeline::run(
        source,
        handler,
        dlq,
        config,
        shutdown.clone(),
    ));

    // Espera a que el consumidor no tenga nada pendiente ni sin confirmar.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let info = consumer.get_info().await.unwrap();
        if info.num_pending == 0 && info.num_ack_pending == 0 && info.delivered.stream_sequence > 0
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "el pipeline no vació el consumidor: {info:?}"
        );
        sleep(Duration::from_millis(100)).await;
    }
    shutdown.cancel();
    running.await.unwrap();

    let stream = js.get_stream(&nats_config.stream).await.unwrap();
    assert_eq!(
        stream.get_info().await.unwrap().state.messages,
        4,
        "3 eventos + 1 venenoso"
    );
    assert_eq!(count_final(&clickhouse, &user).await, 3);

    let dead = js
        .get_stream(&nats_config.dlq_stream)
        .await
        .unwrap()
        .get_last_raw_message_by_subject(&format!(
            "{}.invalid_event",
            nats_config.dlq_subject_prefix
        ))
        .await
        .unwrap();
    assert_eq!(dead.payload.as_ref(), b"{ no es json");
    assert_eq!(
        dead.headers.get("Dlq-Reason").unwrap().as_str(),
        "invalid_event"
    );
    assert_eq!(
        dead.headers.get("Dlq-Original-Subject").unwrap().as_str(),
        subject
    );

    js.delete_stream(&nats_config.stream).await.unwrap();
    js.delete_stream(&nats_config.dlq_stream).await.unwrap();
}
