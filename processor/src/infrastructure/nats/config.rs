//! Configuración de NATS JetStream: stream de eventos, consumidor y DLQ.

use std::time::Duration;

use crate::infrastructure::env::{EnvError, millis_or, parse_or, string_or};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatsConfig {
    pub url: String,

    /// Stream de eventos de entrada.
    pub stream: String,
    pub subjects: Vec<String>,
    /// Los eventos de clientes se publican en `<prefijo>.<source>.<type>`; debe caer dentro
    /// de `subjects`.
    pub publish_prefix: String,
    /// Retención `limits`: los mensajes se conservan este tiempo aunque ya se hayan
    /// confirmado, para poder hacer *replay*. Debe cubrir la peor caída del almacén.
    pub stream_max_age: Duration,
    pub replicas: usize,
    /// Ventana en la que JetStream descarta publicaciones con `Nats-Msg-Id` repetido.
    pub duplicate_window: Duration,

    /// Consumidor durable compartido por todas las réplicas del procesador.
    pub consumer: String,
    /// Plazo para confirmar un mensaje antes de que se reentregue. Debe superar con holgura
    /// [`PipelineConfig::max_hold_time`](crate::infrastructure::pipeline::PipelineConfig::max_hold_time).
    pub ack_wait: Duration,
    /// Mensajes sin confirmar en toda la flota. Debe ser ≥ réplicas × filas por lote × (K + 1).
    pub max_ack_pending: i64,
    /// Mensajes por petición de pull.
    pub fetch_batch: usize,

    /// Stream de mensajes muertos; separado para que el consumidor no los lea.
    pub dlq_stream: String,
    /// Los mensajes se publican en `<prefijo>.<motivo>`.
    pub dlq_subject_prefix: String,
    pub dlq_max_age: Duration,
    /// Ventana de deduplicación de la DLQ. Debe superar `ack_wait`: la copia reentregada de
    /// un mensaje llega, como pronto, `ack_wait` después de la primera.
    pub dlq_duplicate_window: Duration,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: "nats://localhost:4222".into(),
            stream: "EVENTS".into(),
            subjects: vec!["events.>".into()],
            publish_prefix: "events".into(),
            stream_max_age: Duration::from_secs(72 * 3600),
            replicas: 1,
            duplicate_window: Duration::from_secs(120),
            consumer: "processor".into(),
            ack_wait: Duration::from_secs(120),
            max_ack_pending: 500_000,
            fetch_batch: 1_000,
            dlq_stream: "EVENTS_DLQ".into(),
            dlq_subject_prefix: "dlq.events".into(),
            dlq_max_age: Duration::from_secs(30 * 24 * 3600),
            dlq_duplicate_window: Duration::from_secs(10 * 60),
        }
    }
}

impl NatsConfig {
    /// Lee `NATS_URL`, `NATS_STREAM`, `NATS_CONSUMER`, `NATS_REPLICAS`, `NATS_ACK_WAIT_MS`,
    /// `NATS_MAX_ACK_PENDING` y `NATS_FETCH_BATCH`. El resto toma el valor de [`Default`].
    pub fn from_env() -> Result<Self, EnvError> {
        let default = Self::default();
        Ok(Self {
            url: string_or("NATS_URL", default.url),
            stream: string_or("NATS_STREAM", default.stream),
            consumer: string_or("NATS_CONSUMER", default.consumer),
            replicas: parse_or("NATS_REPLICAS", default.replicas)?,
            ack_wait: millis_or("NATS_ACK_WAIT_MS", default.ack_wait)?,
            max_ack_pending: parse_or("NATS_MAX_ACK_PENDING", default.max_ack_pending)?,
            fetch_batch: parse_or("NATS_FETCH_BATCH", default.fetch_batch)?,
            ..default
        })
    }
}
