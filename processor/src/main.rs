//! Procesador: consume eventos de NATS y los persiste en ClickHouse.

use std::sync::Arc;

use data_processor::application::command::RecordEventsHandler;
use data_processor::infrastructure::clickhouse::{ClickHouseConfig, ClickHouseEventRepository};
use data_processor::infrastructure::nats::{self, NatsConfig, NatsDeadLetterQueue};
use data_processor::infrastructure::pipeline::{self, PipelineConfig};
use data_processor::infrastructure::{BoxError, heartbeat, shutdown_signal, telemetry};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let telemetry = telemetry::init("data-processor");
    heartbeat::spawn_from_env();

    let clickhouse = ClickHouseConfig::from_env();
    let nats_config = NatsConfig::from_env()?;
    let pipeline_config = PipelineConfig::from_env()?;
    if pipeline_config.max_hold_time() * 2 > nats_config.ack_wait {
        warn!(
            hold = ?pipeline_config.max_hold_time(),
            ack_wait = ?nats_config.ack_wait,
            "AckWait apenas cubre lote + reintentos: habrá reentregas de lotes aún en curso"
        );
    }

    if nats_config.dlq_duplicate_window <= nats_config.ack_wait {
        warn!(
            dlq_window = ?nats_config.dlq_duplicate_window,
            ack_wait = ?nats_config.ack_wait,
            "la ventana de deduplicación de la DLQ no cubre AckWait: un mensaje reentregado podría quedar dos veces en la DLQ"
        );
    }

    let js = nats::connect(&nats_config).await?;
    nats::ensure_streams(&js, &nats_config).await?;
    let consumer = nats::ensure_consumer(&js, &nats_config).await?;
    let source = nats::deliveries(&consumer, &nats_config).await?;
    let dlq = Arc::new(NatsDeadLetterQueue::new(js, &nats_config));
    let repository = ClickHouseEventRepository::new(clickhouse.client());
    let handler = Arc::new(RecordEventsHandler::new(repository));

    let shutdown = CancellationToken::new();
    let cancel = shutdown.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        info!("cerrando lotes en curso");
        cancel.cancel();
    });

    info!(stream = %nats_config.stream, consumer = %nats_config.consumer, "procesador iniciado");
    pipeline::run(source, handler, dlq, pipeline_config, shutdown).await;
    info!("procesador detenido");
    telemetry.shutdown();
    Ok(())
}
