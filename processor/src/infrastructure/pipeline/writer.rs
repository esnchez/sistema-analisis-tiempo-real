//! Etapa 3: escribe hasta K lotes a la vez y confirma sus mensajes tras persistirlos.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{Semaphore, mpsc};
use tokio::task::JoinSet;
use tracing::{error, warn};

use super::batcher::Batch;
use super::breaker::CircuitBreaker;
use super::delivery::{DeadLetterQueue, DeadLetterReason, Delivery, dead_letter};
use super::metrics::{InsertOutcome, PipelineMetrics};
use super::retry::RetryPolicy;
use crate::application::command::{CommandHandler, RecordEvents};
use crate::application::port::RepositoryError;

/// Dependencias que comparte cada tarea de escritura.
pub(super) struct Writer<H, Q> {
    pub handler: Arc<H>,
    pub dlq: Arc<Q>,
    pub breaker: Arc<CircuitBreaker>,
    pub retry: RetryPolicy,
    pub metrics: Arc<PipelineMetrics>,
}

impl<H, Q> Clone for Writer<H, Q> {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            dlq: self.dlq.clone(),
            breaker: self.breaker.clone(),
            retry: self.retry,
            metrics: self.metrics.clone(),
        }
    }
}

/// Lanza una tarea por lote con como mucho `concurrency` en vuelo. Termina cuando se cierra
/// el canal y todas las tareas han acabado (parada ordenada).
pub(super) async fn writers<D, H, Q>(
    mut batches: mpsc::Receiver<Batch<D>>,
    writer: Writer<H, Q>,
    concurrency: usize,
) where
    D: Delivery,
    H: CommandHandler<RecordEvents, Error = RepositoryError> + Send + Sync + 'static,
    Q: DeadLetterQueue,
{
    let permits: Arc<Semaphore> = Arc::new(Semaphore::new(concurrency));
    let mut tasks: JoinSet<()> = JoinSet::new();
    while let Some(batch) = batches.recv().await {
        // Espera si hay `concurrency` lotes en vuelo: backpressure hacia el batcher.
        let permit = permits
            .clone()
            .acquire_owned()
            .await
            .expect("el semáforo nunca se cierra");
        let writer = writer.clone();
        tasks.spawn(async move {
            let _permit = permit;
            let bytes = batch.deliveries.iter().map(|d| d.payload().len()).sum();
            writer.metrics.batch_started(batch.len(), bytes);
            writer.write(batch).await;
            writer.metrics.batch_finished();
        });
        while let Some(result) = tasks.try_join_next() {
            report(result);
        }
    }
    while let Some(result) = tasks.join_next().await {
        report(result);
    }
}

fn report(result: Result<(), tokio::task::JoinError>) {
    if let Err(error) = result {
        // Sus mensajes siguen sin ack: el broker los reentregará.
        error!(%error, "una tarea de escritura terminó de forma anómala");
    }
}

impl<H, Q> Writer<H, Q>
where
    H: CommandHandler<RecordEvents, Error = RepositoryError>,
    Q: DeadLetterQueue,
{
    /// Persiste el lote y confirma sus mensajes. Si el almacén lo rechaza, lo parte en
    /// mitades hasta aislar los eventos culpables (bisección), que acaban en la DLQ.
    async fn write<D: Delivery>(&self, batch: Batch<D>) {
        let mut pending = vec![batch];
        while let Some(batch) = pending.pop() {
            let command = RecordEvents::new(batch.events.clone());
            match self.retry.run(|| self.insert(command.clone())).await {
                Ok(()) => {
                    self.breaker.record_success();
                    ack_all(&batch.deliveries).await;
                    self.metrics.events_stored(&batch.events);
                }
                Err(RepositoryError::Transient(error)) => {
                    self.breaker.record_failure();
                    self.metrics.batch_abandoned();
                    warn!(
                        %error,
                        rows = batch.len() + pending.iter().map(Batch::len).sum::<usize>(),
                        "reintentos agotados; los mensajes se reentregarán"
                    );
                    // Sin ack: el broker reentrega todo tras `AckWait`.
                    return;
                }
                Err(RepositoryError::Rejected(error)) => {
                    // El almacén respondió: está disponible.
                    self.breaker.record_success();
                    if batch.len() == 1 {
                        let reason = DeadLetterReason::RejectedByStore(error.to_string());
                        dead_letter(&*self.dlq, &batch.deliveries[0], reason, &self.metrics).await;
                    } else {
                        let (left, right) = batch.split();
                        pending.push(right);
                        pending.push(left);
                    }
                }
            }
        }
    }
}

impl<H, Q> Writer<H, Q>
where
    H: CommandHandler<RecordEvents, Error = RepositoryError>,
{
    /// Un intento de `INSERT`, medido.
    async fn insert(&self, command: RecordEvents) -> Result<(), RepositoryError> {
        let started = Instant::now();
        let result = self.handler.handle(command).await;
        let outcome = match &result {
            Ok(()) => InsertOutcome::Ok,
            Err(RepositoryError::Transient(_)) => InsertOutcome::Transient,
            Err(RepositoryError::Rejected(_)) => InsertOutcome::Rejected,
        };
        self.metrics
            .insert_attempt(started.elapsed().as_secs_f64(), outcome);
        result
    }
}

async fn ack_all<D: Delivery>(deliveries: &[D]) {
    for delivery in deliveries {
        if let Err(error) = delivery.ack().await {
            // Ya está persistido: la reentrega solo producirá un duplicado que se descarta.
            warn!(%error, subject = delivery.subject(), "no se pudo confirmar el mensaje");
        }
    }
}
