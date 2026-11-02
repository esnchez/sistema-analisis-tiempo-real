//! Publicación en JetStream de los eventos que envían los clientes.

use std::future::IntoFuture;

use async_nats::HeaderMap;
use async_nats::jetstream::Context;
use futures_util::future::join_all;

use super::NatsConfig;
use crate::application::port::{EventPublisher, PublishError, RawEvent};

/// Longitud máxima de un `event_id` usable como `Nats-Msg-Id`.
const MAX_MSG_ID_LEN: usize = 128;

/// Publica cada evento en `<prefijo>.<source>.<type>` con `Nats-Msg-Id = event_id`.
///
/// Con `Nats-Msg-Id`, JetStream descarta el reintento de un evento ya publicado dentro de
/// su `duplicate_window`. Los eventos sin tipo o `event_id` utilizables se publican igual
/// (en `<prefijo>.unknown` o sin id): el procesador decide qué hacer con ellos.
#[derive(Clone)]
pub struct NatsEventPublisher {
    js: Context,
    prefix: String,
}

impl NatsEventPublisher {
    pub fn new(js: Context, config: &NatsConfig) -> Self {
        Self {
            js,
            prefix: config.publish_prefix.clone(),
        }
    }
}

impl EventPublisher for NatsEventPublisher {
    async fn publish(&self, events: Vec<RawEvent>) -> Result<(), PublishError> {
        // Envía todo y después espera las confirmaciones a la vez: una ida y vuelta por
        // lote en vez de una por evento.
        let mut acks = Vec::with_capacity(events.len());
        for event in events {
            let subject = subject(&self.prefix, event.event_type.as_deref());
            let mut headers = HeaderMap::new();
            if let Some(id) = event.event_id.as_deref().filter(|id| is_valid_msg_id(id)) {
                headers.insert("Nats-Msg-Id", id);
            }
            let ack = self
                .js
                .publish_with_headers(subject, headers, event.payload.into())
                .await
                .map_err(PublishError::new)?;
            acks.push(ack.into_future());
        }
        for ack in join_all(acks).await {
            // Un duplicado (`ack.duplicate`) también es éxito: ya estaba guardado.
            ack.map_err(PublishError::new)?;
        }
        Ok(())
    }
}

/// `<prefijo>.<source>.<type>` si el tipo declarado tiene esa forma con nombres seguros
/// para un subject; si no, `<prefijo>.unknown`.
fn subject(prefix: &str, event_type: Option<&str>) -> String {
    match event_type.and_then(|event_type| event_type.split_once('.')) {
        Some((source, kind)) if is_token(source) && is_token(kind) => {
            format!("{prefix}.{source}.{kind}")
        }
        _ => format!("{prefix}.unknown"),
    }
}

/// Segmento de subject seguro: sin puntos, comodines ni espacios.
fn is_token(segment: &str) -> bool {
    !segment.is_empty()
        && segment.len() <= 64
        && segment
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Una cabecera no admite caracteres de control (saltos de línea).
fn is_valid_msg_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_MSG_ID_LEN && !id.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "publisher_tests.rs"]
mod tests;
