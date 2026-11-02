//! Gateway HTTP contra un NATS JetStream real (sin abrir puertos: `oneshot` sobre el router).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use data_processor::application::command::PublishEventsHandler;
use data_processor::infrastructure::http::{GatewayConfig, router};
use data_processor::infrastructure::nats::{self, NatsConfig, NatsEventPublisher};
use tower::ServiceExt;
use uuid::Uuid;

fn isolated_nats_config() -> NatsConfig {
    let id = Uuid::now_v7().simple().to_string();
    NatsConfig {
        stream: format!("IT_{id}"),
        subjects: vec![format!("it.{id}.>")],
        publish_prefix: format!("it.{id}"),
        dlq_stream: format!("IT_DLQ_{id}"),
        dlq_subject_prefix: format!("it-dlq.{id}"),
        ..NatsConfig::from_env().unwrap()
    }
}

#[tokio::test]
#[ignore = "requiere NATS: docker compose up -d nats"]
async fn publishes_everything_and_drops_client_retries() {
    let config = isolated_nats_config();
    let js = nats::connect(&config).await.unwrap();
    nats::ensure_streams(&js, &config).await.unwrap();
    let handler = Arc::new(PublishEventsHandler::new(NatsEventPublisher::new(
        js.clone(),
        &config,
    )));
    let event = r#"{"event_id":"e-1","source":"user_data","event":{"type":"account_deleted"}}"#;
    // El mismo evento dos veces (reintento del cliente), uno sin tipo y uno que no es objeto.
    let body = format!(r#"[{event}, {event}, {{"event_id":"e-2"}}, "texto"]"#);

    let request = Request::post("/v1/events").body(Body::from(body)).unwrap();
    let response = router(handler, &GatewayConfig::default())
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let mut stream = js.get_stream(&config.stream).await.unwrap();
    let state = stream.info().await.unwrap().state.clone();
    assert_eq!(state.messages, 3, "el reintento de e-1 se descarta");
    let known = stream
        .get_last_raw_message_by_subject(&format!(
            "{}.user_data.account_deleted",
            config.publish_prefix
        ))
        .await
        .unwrap();
    assert_eq!(known.payload.as_ref(), event.as_bytes());
    let unknown = stream
        .get_last_raw_message_by_subject(&format!("{}.unknown", config.publish_prefix))
        .await
        .unwrap();
    assert_eq!(unknown.payload.as_ref(), br#""texto""#);

    js.delete_stream(&config.stream).await.unwrap();
    js.delete_stream(&config.dlq_stream).await.unwrap();
}
