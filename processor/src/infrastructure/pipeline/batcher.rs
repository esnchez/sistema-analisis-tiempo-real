//! Etapa 2: agrupa eventos en lotes por número de filas, bytes o antigüedad.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};

use super::delivery::Delivery;
use crate::domain::event::SourceEvent;

/// Evento validado junto con el mensaje que hay que confirmar cuando se persista.
pub(super) struct Pending<D> {
    pub event: SourceEvent,
    pub delivery: D,
}

/// Lote inmutable. `events[i]` procede de `deliveries[i]`.
pub(super) struct Batch<D> {
    pub events: Arc<[SourceEvent]>,
    pub deliveries: Vec<D>,
}

impl<D> Batch<D> {
    pub fn len(&self) -> usize {
        self.deliveries.len()
    }

    /// Parte el lote en dos mitades (para la bisección).
    pub fn split(self) -> (Self, Self) {
        let mid = self.len() / 2;
        let mut left = self.deliveries;
        let right = left.split_off(mid);
        (
            Self {
                events: self.events[..mid].into(),
                deliveries: left,
            },
            Self {
                events: self.events[mid..].into(),
                deliveries: right,
            },
        )
    }
}

/// Cuándo se cierra un lote: lo primero que ocurra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchLimits {
    pub max_rows: usize,
    /// Suma de los tamaños de los mensajes originales.
    pub max_bytes: usize,
    /// Antigüedad del primer evento del lote. Acota la frescura.
    pub max_age: Duration,
}

impl Default for BatchLimits {
    fn default() -> Self {
        Self {
            max_rows: 50_000,
            max_bytes: 32 * 1024 * 1024,
            max_age: Duration::from_secs(2),
        }
    }
}

/// Recibe eventos hasta que se cierra el canal de entrada; entonces envía el lote abierto
/// y termina, lo que a su vez cierra el canal de los writers.
pub(super) async fn batcher<D: Delivery>(
    mut events: mpsc::Receiver<Pending<D>>,
    batches: mpsc::Sender<Batch<D>>,
    limits: BatchLimits,
) {
    let mut open = OpenBatch::default();
    loop {
        let deadline = open.deadline.unwrap_or_else(Instant::now);
        let full = tokio::select! {
            pending = events.recv() => match pending {
                Some(pending) => open.push(pending, &limits),
                None => break,
            },
            () = sleep_until(deadline), if open.deadline.is_some() => true,
        };
        // `send` espera si los writers están ocupados: backpressure hacia la ingesta.
        if full && batches.send(open.take()).await.is_err() {
            return;
        }
    }
    if !open.events.is_empty() {
        let _ = batches.send(open.take()).await;
    }
}

struct OpenBatch<D> {
    events: Vec<SourceEvent>,
    deliveries: Vec<D>,
    bytes: usize,
    deadline: Option<Instant>,
}

impl<D> Default for OpenBatch<D> {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            deliveries: Vec::new(),
            bytes: 0,
            deadline: None,
        }
    }
}

impl<D: Delivery> OpenBatch<D> {
    /// Añade el evento y devuelve si el lote alcanzó su límite de filas o bytes.
    fn push(&mut self, pending: Pending<D>, limits: &BatchLimits) -> bool {
        if self.deadline.is_none() {
            self.deadline = Some(Instant::now() + limits.max_age);
        }
        self.bytes += pending.delivery.payload().len();
        self.events.push(pending.event);
        self.deliveries.push(pending.delivery);
        self.events.len() >= limits.max_rows || self.bytes >= limits.max_bytes
    }

    fn take(&mut self) -> Batch<D> {
        let open = std::mem::take(self);
        Batch {
            events: open.events.into(),
            deliveries: open.deliveries,
        }
    }
}
