//! Pipeline concurrente de ingesta: broker → lotes → almacén.
//!
//! ```text
//! intake ──mpsc──▶ batcher ──mpsc──▶ writers ×K ──▶ ack
//!   │ inválido                         │ rechazo permanente
//!   ▼                                  ▼
//!  DLQ                          bisección → DLQ
//! ```
//!
//! - Canales acotados y `Semaphore(K)`: memoria limitada y backpressure hasta el broker.
//! - Ack por mensaje y solo tras persistir: los lotes pueden terminar en cualquier orden.
//! - Fallo transitorio: reintentos con backoff; agotados, sin ack (el broker reentrega) y el
//!   circuit breaker detiene la ingesta.
//! - `shutdown`: deja de pedir, cierra el lote abierto, espera las escrituras y termina.
//!
//! No depende del broker: recibe un `Stream` de [`Delivery`]. El adaptador de NATS está en
//! [`crate::infrastructure::nats`]. Diseño en `docs/processing/concurrency.md`.

mod batcher;
mod breaker;
mod delivery;
mod intake;
mod metrics;
mod retry;
mod writer;

use std::sync::Arc;
use std::time::Duration;

use futures_util::Stream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub use batcher::BatchLimits;
pub use breaker::BreakerConfig;
pub use delivery::{DeadLetterQueue, DeadLetterReason, Delivery};
pub use retry::RetryPolicy;

use crate::application::command::{CommandHandler, RecordEvents};
use crate::application::port::RepositoryError;
use crate::domain::event::EventTimeWindow;
#[cfg(test)]
use crate::infrastructure::BoxError;
use crate::infrastructure::env::{EnvError, millis_or, parse_or};

/// Parámetros del pipeline. Ver los valores por defecto en cada tipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineConfig {
    pub batch: BatchLimits,
    /// Lotes escribiéndose a la vez (K).
    pub writers: usize,
    /// Capacidad del canal entre ingesta y batcher, en eventos.
    pub channel_capacity: usize,
    pub retry: RetryPolicy,
    pub breaker: BreakerConfig,
    pub time_window: EventTimeWindow,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            batch: BatchLimits::default(),
            writers: 2,
            channel_capacity: 10_000,
            retry: RetryPolicy::default(),
            breaker: BreakerConfig::default(),
            time_window: EventTimeWindow::default(),
        }
    }
}

impl PipelineConfig {
    /// Lee `PIPELINE_BATCH_ROWS`, `PIPELINE_BATCH_BYTES`, `PIPELINE_BATCH_AGE_MS` y
    /// `PIPELINE_WRITERS`. El resto toma el valor de [`Default`].
    pub fn from_env() -> Result<Self, EnvError> {
        let default = Self::default();
        Ok(Self {
            batch: BatchLimits {
                max_rows: parse_or("PIPELINE_BATCH_ROWS", default.batch.max_rows)?,
                max_bytes: parse_or("PIPELINE_BATCH_BYTES", default.batch.max_bytes)?,
                max_age: millis_or("PIPELINE_BATCH_AGE_MS", default.batch.max_age)?,
            },
            writers: parse_or("PIPELINE_WRITERS", default.writers)?,
            ..default
        })
    }

    /// Tiempo aproximado que un lote retiene sus mensajes sin ack en el peor caso
    /// (llenado + reintentos). El `AckWait` del broker debe superarlo con holgura.
    pub fn max_hold_time(&self) -> Duration {
        self.batch.max_age + self.retry.max_total_backoff()
    }
}

/// Ejecuta el pipeline hasta que `source` se agote o se cancele `shutdown`, y espera a que
/// todos los lotes en curso terminen.
pub async fn run<S, D, H, Q>(
    source: S,
    handler: Arc<H>,
    dlq: Arc<Q>,
    config: PipelineConfig,
    shutdown: CancellationToken,
) where
    S: Stream<Item = D> + Unpin,
    D: Delivery,
    H: CommandHandler<RecordEvents, Error = RepositoryError> + Send + Sync + 'static,
    Q: DeadLetterQueue,
{
    let breaker = Arc::new(breaker::CircuitBreaker::new(config.breaker));
    let metrics = Arc::new(metrics::PipelineMetrics::new(breaker.clone()));
    let (events_tx, events_rx) = mpsc::channel(config.channel_capacity.max(1));
    // Capacidad 1: como mucho un lote listo esperando a un writer libre.
    let (batches_tx, batches_rx) = mpsc::channel(1);
    let writer = writer::Writer {
        handler,
        dlq: dlq.clone(),
        breaker: breaker.clone(),
        retry: config.retry,
        metrics: metrics.clone(),
    };

    tokio::join!(
        intake::intake(
            source,
            events_tx,
            &*dlq,
            config.time_window,
            &breaker,
            &metrics,
            shutdown
        ),
        batcher::batcher(events_rx, batches_tx, config.batch),
        writer::writers(batches_rx, writer, config.writers.max(1)),
    );
}

#[cfg(test)]
mod tests;
