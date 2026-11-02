//! Gateway de ingesta: recibe eventos de los clientes por HTTP y los publica en NATS.

use std::sync::Arc;

use data_processor::application::command::PublishEventsHandler;
use data_processor::infrastructure::http::{self, GatewayConfig};
use data_processor::infrastructure::nats::{self, NatsConfig, NatsEventPublisher};
use data_processor::infrastructure::{BoxError, shutdown_signal, telemetry};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let telemetry = telemetry::init("event-gateway");

    let nats_config = NatsConfig::from_env()?;
    let gateway_config = GatewayConfig::from_env()?;

    let js = nats::connect(&nats_config).await?;
    nats::ensure_streams(&js, &nats_config).await?;
    let publisher = NatsEventPublisher::new(js, &nats_config);
    let handler = Arc::new(PublishEventsHandler::new(publisher));

    let listener = TcpListener::bind(gateway_config.listen_addr).await?;
    info!(addr = %gateway_config.listen_addr, "gateway iniciado");
    // Al recibir la señal deja de aceptar conexiones y termina las peticiones en curso.
    axum::serve(listener, http::router(handler, &gateway_config))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    info!("gateway detenido");
    telemetry.shutdown();
    Ok(())
}
