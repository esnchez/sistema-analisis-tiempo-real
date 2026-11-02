//! Métricas HTTP del gateway (OpenTelemetry). Sin `MeterProvider` configurado no hacen nada.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram};

pub(super) struct HttpMetrics {
    duration: Histogram<f64>,
    accepted: Counter<u64>,
}

impl HttpMetrics {
    pub fn new() -> Self {
        let meter = global::meter("event-gateway");
        Self {
            // Nombre y atributos de las convenciones semánticas de OTel para HTTP.
            duration: meter
                .f64_histogram("http.server.request.duration")
                .with_unit("s")
                .with_description("Duración de las peticiones HTTP")
                .with_boundaries(vec![
                    0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.5, 1.0, 2.5, 5.0,
                ])
                .build(),
            accepted: meter
                .u64_counter("gateway.events.accepted")
                .with_description("Eventos publicados y confirmados por JetStream")
                .build(),
        }
    }

    pub fn events_accepted(&self, count: usize) {
        self.accepted.add(count as u64, &[]);
    }
}

/// Middleware: mide cada petición por ruta (plantilla, no URL: cardinalidad acotada),
/// método y código de respuesta.
pub(super) async fn record(
    State(metrics): State<Arc<HttpMetrics>>,
    request: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "unmatched".to_owned(), |path| path.as_str().to_owned());
    let method = request.method().as_str().to_owned();
    let response = next.run(request).await;
    metrics.duration.record(
        started.elapsed().as_secs_f64(),
        &[
            KeyValue::new("http.route", route),
            KeyValue::new("http.request.method", method),
            KeyValue::new(
                "http.response.status_code",
                i64::from(response.status().as_u16()),
            ),
        ],
    );
    response
}
