//! Métricas del pipeline (OpenTelemetry).
//!
//! Se registran en el `MeterProvider` global: si no se configuró ninguno
//! ([`crate::infrastructure::telemetry`]), los instrumentos no hacen nada. Por eso hay que
//! crearlas **después** de iniciar la telemetría, al arrancar [`super::run`].
//!
//! Nombres con puntos (convención OTel); en Prometheus aparecen con `_` y el sufijo de
//! unidad (`processor_insert_duration_seconds`). Catálogo en `docs/operaciones/observabilidad.md`.

use std::sync::Arc;

use chrono::Utc;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram, ObservableGauge, UpDownCounter};

use super::breaker::CircuitBreaker;
use crate::domain::event::SourceEvent;

const METER: &str = "data-processor";

pub(super) struct PipelineMetrics {
    received: Counter<u64>,
    stored: Counter<u64>,
    dead_letters: Counter<u64>,
    abandoned_batches: Counter<u64>,
    batch_rows: Histogram<u64>,
    batch_bytes: Histogram<u64>,
    insert_duration: Histogram<f64>,
    freshness: Histogram<f64>,
    busy_writers: UpDownCounter<i64>,
    /// Se lee en cada exportación; hay que mantenerlo vivo mientras corre el pipeline.
    _breaker_open: ObservableGauge<u64>,
}

/// Lote en escritura; al soltarse descuenta `processor.writers.busy`.
pub(super) struct BatchInProgress<'a>(&'a PipelineMetrics);

impl Drop for BatchInProgress<'_> {
    fn drop(&mut self) {
        self.0.busy_writers.add(-1, &[]);
    }
}

/// Resultado de un intento de `INSERT`.
#[derive(Debug, Clone, Copy)]
pub(super) enum InsertOutcome {
    Ok,
    Transient,
    Rejected,
}

impl InsertOutcome {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Transient => "transient",
            Self::Rejected => "rejected",
        }
    }
}

impl PipelineMetrics {
    pub fn new(breaker: Arc<CircuitBreaker>) -> Self {
        let meter = global::meter(METER);
        Self {
            received: meter
                .u64_counter("processor.events.received")
                .with_description("Eventos válidos leídos del broker")
                .build(),
            stored: meter
                .u64_counter("processor.events.stored")
                .with_description("Eventos persistidos y confirmados")
                .build(),
            dead_letters: meter
                .u64_counter("processor.dead_letters")
                .with_description("Mensajes apartados en la DLQ, por motivo")
                .build(),
            abandoned_batches: meter
                .u64_counter("processor.batches.abandoned")
                .with_description("Lotes que agotaron los reintentos; el broker los reentregará")
                .build(),
            batch_rows: meter
                .u64_histogram("processor.batch.rows")
                .with_description("Filas por lote")
                .with_boundaries(vec![
                    1.0, 10.0, 100.0, 1_000.0, 5_000.0, 10_000.0, 25_000.0, 50_000.0,
                ])
                .build(),
            batch_bytes: meter
                .u64_histogram("processor.batch.size")
                .with_unit("By")
                .with_description("Bytes (mensajes originales) por lote")
                .with_boundaries(
                    (0..=8).map(|exp| 1024.0 * 4f64.powi(exp)).collect(), // 1 KiB … 64 MiB
                )
                .build(),
            insert_duration: meter
                .f64_histogram("processor.insert.duration")
                .with_unit("s")
                .with_description("Duración de cada intento de INSERT, por resultado")
                .with_boundaries(vec![
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
                ])
                .build(),
            freshness: meter
                .f64_histogram("processor.event.freshness")
                .with_unit("s")
                .with_description("Desde occurred_at hasta quedar persistido")
                .with_boundaries(vec![
                    0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 10.0, 30.0, 60.0, 300.0, 3_600.0,
                ])
                .build(),
            busy_writers: meter
                .i64_up_down_counter("processor.writers.busy")
                .with_description("Lotes escribiéndose ahora mismo (≤ K)")
                .build(),
            _breaker_open: meter
                .u64_observable_gauge("processor.breaker.open")
                .with_description("1 si el circuit breaker está abierto (sin consumir)")
                .with_callback(move |observer| observer.observe(breaker.is_open().into(), &[]))
                .build(),
        }
    }

    pub fn event_received(&self) {
        self.received.add(1, &[]);
    }

    pub fn dead_lettered(&self, reason: &'static str) {
        self.dead_letters.add(1, &[KeyValue::new("reason", reason)]);
    }

    /// Marca un lote como en escritura hasta que se suelte el valor devuelto. Al ser un
    /// guard, `processor.writers.busy` baja también si la tarea entra en pánico.
    #[must_use = "el lote cuenta como en escritura hasta que se suelta el guard"]
    pub fn batch_started(&self, rows: usize, bytes: usize) -> BatchInProgress<'_> {
        self.busy_writers.add(1, &[]);
        self.batch_rows.record(rows as u64, &[]);
        self.batch_bytes.record(bytes as u64, &[]);
        BatchInProgress(self)
    }

    pub fn batch_abandoned(&self) {
        self.abandoned_batches.add(1, &[]);
    }

    pub fn insert_attempt(&self, seconds: f64, outcome: InsertOutcome) {
        self.insert_duration
            .record(seconds, &[KeyValue::new("outcome", outcome.label())]);
    }

    /// Eventos persistidos: los cuenta y registra su frescura.
    pub fn events_stored(&self, events: &[SourceEvent]) {
        self.stored.add(events.len() as u64, &[]);
        let now = Utc::now();
        for event in events {
            let age = (now - event.occurred_at()).as_seconds_f64();
            self.freshness.record(age.max(0.0), &[]);
        }
    }
}
