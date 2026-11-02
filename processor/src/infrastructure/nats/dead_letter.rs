//! DLQ sobre un stream de JetStream.

use async_nats::HeaderMap;
use async_nats::jetstream::Context;

use super::NatsConfig;
use crate::infrastructure::BoxError;
use crate::infrastructure::pipeline::{DeadLetterQueue, DeadLetterReason, Delivery};

/// Publica el mensaje original en `<prefijo>.<motivo>` con el motivo en cabeceras
/// (`Dlq-Reason`, `Dlq-Detail`, `Dlq-Original-Subject`) y espera la confirmación del stream.
///
/// Idempotente: `Nats-Msg-Id` = posición del mensaje original (`<stream>:<secuencia>`). Si
/// el mensaje se reentrega antes de terminarse (p. ej. el pod cae entre la DLQ y el `term`),
/// la segunda copia se descarta dentro de la `duplicate_window` del stream de la DLQ.
#[derive(Clone)]
pub struct NatsDeadLetterQueue {
    js: Context,
    subject_prefix: String,
}

impl NatsDeadLetterQueue {
    pub fn new(js: Context, config: &NatsConfig) -> Self {
        Self {
            js,
            subject_prefix: config.dlq_subject_prefix.clone(),
        }
    }
}

impl DeadLetterQueue for NatsDeadLetterQueue {
    async fn send<D: Delivery>(
        &self,
        delivery: &D,
        reason: &DeadLetterReason,
    ) -> Result<(), BoxError> {
        let mut headers = HeaderMap::new();
        headers.insert("Dlq-Reason", reason.kind());
        headers.insert("Dlq-Detail", single_line(reason.detail()).as_str());
        headers.insert("Dlq-Original-Subject", delivery.subject());
        if let Some(id) = delivery.id() {
            headers.insert("Nats-Msg-Id", id.as_str());
        }
        let subject = format!("{}.{}", self.subject_prefix, reason.kind());
        self.js
            .publish_with_headers(subject, headers, delivery.payload().to_vec().into())
            .await?
            .await?;
        Ok(())
    }
}

/// Las cabeceras no admiten saltos de línea.
fn single_line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}
