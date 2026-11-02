//! Gateway HTTP: punto de entrada de los eventos de los clientes.
//!
//! ```text
//! POST /v1/events   [evento, evento, ...]  (o un único objeto)
//!   202 {"accepted": n}   publicados y guardados por JetStream
//!   400                   el cuerpo no es JSON
//!   413                   cuerpo o número de eventos por encima del límite
//!   503                   el bus no confirmó: reintentar el lote completo
//! GET  /health            200
//! ```
//!
//! Tolerante a propósito: no valida el contenido de los eventos. Los clientes pueden enviar
//! eventos erróneos u obsoletos; se publican igual y el procesador los aparta en la DLQ.

mod config;
mod metrics;
mod raw_event;

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use serde_json::value::RawValue;
use tracing::warn;

pub use config::GatewayConfig;

use crate::application::command::{CommandHandler, PublishEvents};
use crate::application::port::PublishError;

struct AppState<H> {
    handler: Arc<H>,
    max_events: usize,
    metrics: Arc<metrics::HttpMetrics>,
}

/// Rutas del gateway sobre el manejador de [`PublishEvents`].
pub fn router<H>(handler: Arc<H>, config: &GatewayConfig) -> Router
where
    H: CommandHandler<PublishEvents, Error = PublishError> + Send + Sync + 'static,
{
    let metrics = Arc::new(metrics::HttpMetrics::new());
    let state = Arc::new(AppState {
        handler,
        max_events: config.max_events,
        metrics: metrics.clone(),
    });
    Router::new()
        .route("/v1/events", post(publish_events::<H>))
        .route("/health", get(|| async { StatusCode::OK }))
        .layer(DefaultBodyLimit::max(config.max_body_bytes))
        // Por fuera del límite de cuerpo: también mide los 413.
        .layer(middleware::from_fn_with_state(metrics, metrics::record))
        .with_state(state)
}

async fn publish_events<H>(State(state): State<Arc<AppState<H>>>, body: Bytes) -> Response
where
    H: CommandHandler<PublishEvents, Error = PublishError> + Send + Sync + 'static,
{
    let Some(elements) = elements(&body) else {
        return error(StatusCode::BAD_REQUEST, "el cuerpo debe ser JSON");
    };
    if elements.len() > state.max_events {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!("como mucho {} eventos por petición", state.max_events),
        );
    }

    let accepted = elements.len();
    let events = elements.into_iter().map(raw_event::raw_event).collect();
    match state.handler.handle(PublishEvents::new(events)).await {
        Ok(()) => {
            state.metrics.events_accepted(accepted);
            (StatusCode::ACCEPTED, Json(json!({ "accepted": accepted }))).into_response()
        }
        Err(publish_error) => {
            warn!(error = %publish_error, "no se pudieron publicar los eventos");
            let mut response = error(StatusCode::SERVICE_UNAVAILABLE, "reintentar más tarde");
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
            response
        }
    }
}

/// Elementos del cuerpo: los de un array JSON o, si es otro valor, ese valor solo.
/// `None` si el cuerpo no es JSON.
fn elements(body: &[u8]) -> Option<Vec<&RawValue>> {
    if let Ok(elements) = serde_json::from_slice::<Vec<&RawValue>>(body) {
        return Some(elements);
    }
    serde_json::from_slice::<&RawValue>(body)
        .ok()
        .map(|single| vec![single])
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests;
