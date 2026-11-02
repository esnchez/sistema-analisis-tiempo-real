//! Pruebas del router con un manejador en memoria (sin abrir puertos).

use std::io;
use std::sync::Mutex;

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use super::*;
use crate::application::port::RawEvent;

#[derive(Default)]
struct FakeHandler {
    published: Mutex<Vec<RawEvent>>,
    fail: bool,
}

impl CommandHandler<PublishEvents> for FakeHandler {
    type Error = PublishError;

    async fn handle(&self, command: PublishEvents) -> Result<(), PublishError> {
        if self.fail {
            return Err(PublishError::new(io::Error::other("sin bus")));
        }
        self.published.lock().unwrap().extend(command.events);
        Ok(())
    }
}

async fn post(
    handler: Arc<FakeHandler>,
    config: &GatewayConfig,
    body: &str,
) -> (StatusCode, String) {
    let request = Request::post("/v1/events")
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    let response = router(handler, config).oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn accepts_a_batch_without_validating_its_content() {
    let handler = Arc::new(FakeHandler::default());
    // Uno válido en apariencia, uno sin campos y uno que ni siquiera es un objeto.
    let body = r#"[{"event_id":"e-1","source":"user_data","event":{"type":"account_deleted"}},
                   {}, "texto"]"#;

    let (status, response) = post(handler.clone(), &GatewayConfig::default(), body).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(response, r#"{"accepted":3}"#);
    let published = handler.published.lock().unwrap();
    assert_eq!(published.len(), 3);
    assert_eq!(published[0].event_id.as_deref(), Some("e-1"));
    assert_eq!(published[1].payload, b"{}");
    assert_eq!(published[2].payload, br#""texto""#);
}

#[tokio::test]
async fn accepts_a_single_event_outside_an_array() {
    let handler = Arc::new(FakeHandler::default());

    let (status, _) = post(handler.clone(), &GatewayConfig::default(), r#"{"a":1}"#).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(handler.published.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn rejects_a_body_that_is_not_json() {
    let (status, _) = post(Arc::default(), &GatewayConfig::default(), "{ no es json").await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_too_many_events() {
    let config = GatewayConfig {
        max_events: 2,
        ..GatewayConfig::default()
    };

    let (status, _) = post(Arc::default(), &config, "[1, 2, 3]").await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn rejects_a_body_over_the_size_limit() {
    let config = GatewayConfig {
        max_body_bytes: 16,
        ..GatewayConfig::default()
    };

    let (status, _) = post(
        Arc::default(),
        &config,
        &format!("[\"{}\"]", "x".repeat(32)),
    )
    .await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn bus_failure_asks_the_client_to_retry() {
    let handler = Arc::new(FakeHandler {
        fail: true,
        ..FakeHandler::default()
    });

    let (status, _) = post(handler, &GatewayConfig::default(), "[{}]").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}
