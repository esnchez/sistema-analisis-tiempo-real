//! Publicación de eventos de clientes en el bus de mensajería.

use std::error::Error;

use thiserror::Error;

/// Evento tal como lo envió un cliente, **sin validar**.
///
/// Los clientes (web, apps, SDKs desactualizados) pueden mandar eventos erróneos u
/// obsoletos. No se rechazan en la entrada: se publican tal cual y el procesador los valida
/// y aparta en la DLQ. Solo se extrae, si existe, lo necesario para enrutar y deduplicar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEvent {
    /// JSON original del evento.
    pub payload: Vec<u8>,
    /// `event_id` declarado por el cliente: clave de deduplicación de los reintentos.
    pub event_id: Option<String>,
    /// Tipo declarado, `<source>.<type>` (p. ej. `user_behavior.content_viewed`).
    pub event_type: Option<String>,
}

/// Bus en el que se publican los eventos para su procesamiento.
pub trait EventPublisher: Send + Sync {
    /// Publica todos los eventos y vuelve cuando el bus los ha guardado de forma durable.
    ///
    /// Si falla, puede que parte se haya publicado: el cliente debe reintentar el lote
    /// completo, y el bus descarta los ya publicados por su `event_id`.
    fn publish(
        &self,
        events: Vec<RawEvent>,
    ) -> impl Future<Output = Result<(), PublishError>> + Send;
}

/// El bus no confirmó la publicación. Reintentar más tarde.
#[derive(Debug, Error)]
#[error("no se pudieron publicar los eventos")]
pub struct PublishError(#[source] Box<dyn Error + Send + Sync>);

impl PublishError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
