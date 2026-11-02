//! Flujo de mensajes del consumidor hacia el pipeline.

use async_nats::jetstream::consumer::PullConsumer;
use async_nats::jetstream::{self, AckKind};
use futures_util::{Stream, StreamExt};
use tracing::warn;

use super::NatsConfig;
use crate::infrastructure::BoxError;
use crate::infrastructure::pipeline::Delivery;

/// Flujo continuo de mensajes del consumidor. Pide `fetch_batch` mensajes cada vez y solo
/// vuelve a pedir cuando se consumen: si el pipeline se frena, deja de pedir.
pub async fn deliveries(
    consumer: &PullConsumer,
    config: &NatsConfig,
) -> Result<impl Stream<Item = JetStreamDelivery> + Unpin + Send + use<>, BoxError> {
    let messages = consumer
        .stream()
        .max_messages_per_batch(config.fetch_batch)
        .messages()
        .await?;
    Ok(messages
        .filter_map(|message| async move {
            match message {
                Ok(message) => Some(JetStreamDelivery(message)),
                Err(error) => {
                    // Errores del pull (heartbeat perdido, reconexión): el flujo continúa.
                    warn!(%error, "error recibiendo de JetStream");
                    None
                }
            }
        })
        .boxed())
}

/// Mensaje de JetStream pendiente de confirmar.
pub struct JetStreamDelivery(jetstream::Message);

impl Delivery for JetStreamDelivery {
    fn subject(&self) -> &str {
        self.0.subject.as_str()
    }

    fn payload(&self) -> &[u8] {
        &self.0.payload
    }

    /// `<stream>:<secuencia>`: la posición del mensaje en el stream no cambia al reentregarlo.
    fn id(&self) -> Option<String> {
        let info = self.0.info().ok()?;
        Some(format!("{}:{}", info.stream, info.stream_sequence))
    }

    async fn ack(&self) -> Result<(), BoxError> {
        self.0.ack().await
    }

    async fn term(&self) -> Result<(), BoxError> {
        self.0.ack_with(AckKind::Term).await
    }
}
