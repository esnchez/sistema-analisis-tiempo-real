//! Conexión y topología de JetStream: streams y consumidor durable.
//!
//! Todo es idempotente (`create_or_update`): cada réplica de cada servicio lo ejecuta al
//! arrancar y aplica la misma configuración, sin depender del orden de arranque.

use async_nats::jetstream::consumer::{AckPolicy, PullConsumer, pull};
use async_nats::jetstream::stream::{self, RetentionPolicy, StorageType};
use async_nats::jetstream::{self, Context};

use super::NatsConfig;
use crate::infrastructure::BoxError;

/// Conecta con el servidor y devuelve el contexto de JetStream.
pub async fn connect(config: &NatsConfig) -> Result<Context, BoxError> {
    let client = async_nats::connect(&config.url).await?;
    Ok(jetstream::new(client))
}

/// Crea o actualiza el stream de eventos y el de la DLQ. Lo necesitan quien publica
/// (gateway) y quien consume (procesador).
pub async fn ensure_streams(js: &Context, config: &NatsConfig) -> Result<(), BoxError> {
    js.create_or_update_stream(stream::Config {
        name: config.stream.clone(),
        subjects: config.subjects.clone(),
        retention: RetentionPolicy::Limits,
        storage: StorageType::File,
        max_age: config.stream_max_age,
        num_replicas: config.replicas,
        duplicate_window: config.duplicate_window,
        ..Default::default()
    })
    .await?;
    js.create_or_update_stream(stream::Config {
        name: config.dlq_stream.clone(),
        subjects: vec![format!("{}.>", config.dlq_subject_prefix)],
        retention: RetentionPolicy::Limits,
        storage: StorageType::File,
        max_age: config.dlq_max_age,
        num_replicas: config.replicas,
        duplicate_window: config.dlq_duplicate_window,
        ..Default::default()
    })
    .await?;
    Ok(())
}

/// Crea o actualiza el consumidor durable del procesador (requiere los streams).
pub async fn ensure_consumer(js: &Context, config: &NatsConfig) -> Result<PullConsumer, BoxError> {
    let consumer = js
        .get_stream(&config.stream)
        .await?
        .create_consumer(pull::Config {
            durable_name: Some(config.consumer.clone()),
            // Ack por mensaje: los lotes concurrentes terminan en cualquier orden.
            ack_policy: AckPolicy::Explicit,
            ack_wait: config.ack_wait,
            // Sin límite: los inválidos van a la DLQ de forma explícita y los fallos
            // transitorios deben reintentarse hasta que el almacén vuelva. Sin `backoff`:
            // sus valores sustituirían a `ack_wait` y reentregarían lotes aún en curso.
            max_deliver: -1,
            max_ack_pending: config.max_ack_pending,
            ..Default::default()
        })
        .await?;
    Ok(consumer)
}
