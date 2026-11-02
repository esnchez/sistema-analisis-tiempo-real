//! Contratos con el broker: mensajes pendientes de confirmar y cola de mensajes muertos (DLQ).

use std::fmt;

use tracing::{error, warn};

use super::metrics::PipelineMetrics;
use crate::infrastructure::BoxError;

/// Mensaje recibido del broker y aún no confirmado.
pub trait Delivery: Send + Sync + 'static {
    /// Subject (tema) en el que se publicó.
    fn subject(&self) -> &str;

    fn payload(&self) -> &[u8];

    /// Identificador estable del mensaje en el broker: el mismo en cada reentrega. Permite
    /// que la DLQ descarte la segunda copia de un mensaje reentregado. `None` si el broker
    /// no lo ofrece.
    fn id(&self) -> Option<String> {
        None
    }

    /// Confirma el mensaje: el broker no lo volverá a entregar.
    fn ack(&self) -> impl Future<Output = Result<(), BoxError>> + Send;

    /// Descarta el mensaje para siempre sin darlo por procesado (tras apartarlo en la DLQ).
    fn term(&self) -> impl Future<Output = Result<(), BoxError>> + Send;
}

/// Motivo por el que un mensaje se aparta en la DLQ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadLetterReason {
    /// No es un evento válido: JSON mal formado o invariante del dominio incumplida.
    InvalidEvent(String),
    /// `occurred_at` fuera de la ventana aceptada.
    OutOfTimeWindow(String),
    /// El almacén rechazó el evento incluso aislado en un lote de uno.
    RejectedByStore(String),
}

impl DeadLetterReason {
    /// Identificador estable del motivo, apto para subjects y métricas.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidEvent(_) => "invalid_event",
            Self::OutOfTimeWindow(_) => "out_of_time_window",
            Self::RejectedByStore(_) => "rejected_by_store",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::InvalidEvent(detail)
            | Self::OutOfTimeWindow(detail)
            | Self::RejectedByStore(detail) => detail,
        }
    }
}

impl fmt::Display for DeadLetterReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind(), self.detail())
    }
}

/// Destino de los mensajes que no se pueden procesar.
pub trait DeadLetterQueue: Send + Sync + 'static {
    /// Guarda el mensaje original junto con el motivo. Solo devuelve `Ok` si quedó persistido.
    fn send<D: Delivery>(
        &self,
        delivery: &D,
        reason: &DeadLetterReason,
    ) -> impl Future<Output = Result<(), BoxError>> + Send;
}

/// Aparta un mensaje: lo publica en la DLQ y, solo si quedó guardado, lo termina.
///
/// Si la DLQ falla, el mensaje no se toca: el broker lo reentregará y se volverá a intentar.
pub(super) async fn dead_letter<D: Delivery, Q: DeadLetterQueue>(
    dlq: &Q,
    delivery: &D,
    reason: DeadLetterReason,
    metrics: &PipelineMetrics,
) {
    warn!(subject = delivery.subject(), %reason, "mensaje apartado en la DLQ");
    if let Err(error) = dlq.send(delivery, &reason).await {
        error!(%error, "no se pudo publicar en la DLQ; el mensaje se reentregará");
        return;
    }
    metrics.dead_lettered(reason.kind());
    if let Err(error) = delivery.term().await {
        warn!(%error, "no se pudo terminar el mensaje; se reentregará y volverá a la DLQ");
    }
}
