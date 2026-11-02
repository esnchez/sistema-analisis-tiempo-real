//! Etapa 1: pide mensajes al broker, los decodifica y valida, y aparta los inválidos.

use chrono::Utc;
use futures_util::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::batcher::Pending;
use super::breaker::CircuitBreaker;
use super::delivery::{DeadLetterQueue, DeadLetterReason, Delivery, dead_letter};
use super::metrics::PipelineMetrics;
use crate::domain::event::{EventTimeWindow, SourceEvent};

/// Consume `source` hasta que se agote o se cancele `shutdown`.
///
/// Al terminar suelta el emisor: eso indica al batcher que cierre el último lote.
pub(super) async fn intake<S, D, Q>(
    mut source: S,
    events: mpsc::Sender<Pending<D>>,
    dlq: &Q,
    window: EventTimeWindow,
    breaker: &CircuitBreaker,
    metrics: &PipelineMetrics,
    shutdown: CancellationToken,
) where
    S: Stream<Item = D> + Unpin,
    D: Delivery,
    Q: DeadLetterQueue,
{
    loop {
        // Con el circuito abierto no se piden mensajes: el backlog se queda en el broker.
        tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            () = breaker.wait_until_closed() => {}
        }
        let delivery = tokio::select! {
            biased;
            () = shutdown.cancelled() => break,
            next = source.next() => match next {
                Some(delivery) => delivery,
                None => break,
            },
        };

        match decode(&delivery, &window) {
            // `send` espera si el canal está lleno: backpressure hacia el broker.
            Ok(event) => {
                metrics.event_received();
                if events.send(Pending { event, delivery }).await.is_err() {
                    break;
                }
            }
            Err(reason) => dead_letter(dlq, &delivery, reason, metrics).await,
        }
    }
}

/// Convierte el mensaje en un evento de dominio válido y dentro de la ventana de tiempo.
fn decode<D: Delivery>(
    delivery: &D,
    window: &EventTimeWindow,
) -> Result<SourceEvent, DeadLetterReason> {
    let event: SourceEvent = serde_json::from_slice(delivery.payload())
        .map_err(|error| DeadLetterReason::InvalidEvent(error.to_string()))?;
    window
        .check(event.occurred_at(), Utc::now())
        .map_err(|error| DeadLetterReason::OutOfTimeWindow(error.to_string()))?;
    Ok(event)
}
