//! DLQ sobre JetStream real: un mensaje reentregado no se duplica en la DLQ.

use std::time::Duration;

use async_nats::jetstream::Context;
use data_processor::infrastructure::nats::{self, NatsConfig, NatsDeadLetterQueue};
use data_processor::infrastructure::pipeline::{DeadLetterQueue, DeadLetterReason, Delivery};
use futures_util::StreamExt;
use tokio::time::timeout;
use uuid::Uuid;

/// Streams exclusivos de esta ejecución y un `AckWait` corto para provocar la reentrega.
fn isolated_config() -> NatsConfig {
    let id = Uuid::now_v7().simple().to_string();
    NatsConfig {
        stream: format!("IT_{id}"),
        subjects: vec![format!("it.{id}.>")],
        dlq_stream: format!("IT_DLQ_{id}"),
        dlq_subject_prefix: format!("it-dlq.{id}"),
        ack_wait: Duration::from_secs(1),
        ..NatsConfig::from_env().unwrap()
    }
}

async fn dlq_messages(js: &Context, config: &NatsConfig) -> u64 {
    let stream = js.get_stream(&config.dlq_stream).await.unwrap();
    stream.get_info().await.unwrap().state.messages
}

#[tokio::test]
#[ignore = "requiere NATS: docker compose up -d"]
async fn redelivered_message_is_dead_lettered_once() {
    let config = isolated_config();
    let js = nats::connect(&config).await.unwrap();
    nats::ensure_streams(&js, &config).await.unwrap();
    let consumer = nats::ensure_consumer(&js, &config).await.unwrap();
    let subject = config.subjects[0].replace('>', "unknown");
    js.publish(subject, b"{ no es json".to_vec().into())
        .await
        .unwrap()
        .await
        .unwrap();

    let dlq = NatsDeadLetterQueue::new(js.clone(), &config);
    let reason = DeadLetterReason::InvalidEvent("prueba".into());
    let mut deliveries = nats::deliveries(&consumer, &config).await.unwrap();
    let next = Duration::from_secs(10);

    // Primera entrega: se aparta en la DLQ, pero se "pierde" el `term` (p. ej. el pod cae).
    let first = timeout(next, deliveries.next()).await.unwrap().unwrap();
    dlq.send(&first, &reason).await.unwrap();

    // Vence `AckWait`: JetStream reentrega el mismo mensaje y se vuelve a apartar.
    let second = timeout(next, deliveries.next()).await.unwrap().unwrap();
    assert_eq!(second.id(), first.id(), "mismo mensaje, mismo id");
    assert!(first.id().is_some());
    dlq.send(&second, &reason).await.unwrap();
    second.term().await.unwrap();

    assert_eq!(
        dlq_messages(&js, &config).await,
        1,
        "la copia reentregada se descarta"
    );

    js.delete_stream(&config.stream).await.unwrap();
    js.delete_stream(&config.dlq_stream).await.unwrap();
}
