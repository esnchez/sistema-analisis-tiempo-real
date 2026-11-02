//! Logs (`tracing`) y métricas (OpenTelemetry → OTLP) de los binarios.
//!
//! Las métricas se **empujan** por OTLP/gRPC a un OpenTelemetry Collector; los servicios no
//! abren ningún puerto para ellas. Solo se activan si existe `OTEL_EXPORTER_OTLP_ENDPOINT`
//! (o `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`); si no, los instrumentos no hacen nada.
//!
//! Variables estándar de OpenTelemetry que se respetan:
//!
//! | Variable | Efecto |
//! |---|---|
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | colector, p. ej. `http://otel-collector:4317` |
//! | `OTEL_METRIC_EXPORT_INTERVAL` | ms entre exportaciones (defecto 60 000) |
//! | `OTEL_RESOURCE_ATTRIBUTES` | atributos extra, p. ej. `service.instance.id=<pod>` |

use std::env;
use std::io::IsTerminal;

use opentelemetry::global;
use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

const ENDPOINT_VARS: [&str; 2] = [
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
];

/// Mantiene viva la exportación. Llamar a [`Telemetry::shutdown`] al salir para enviar
/// las últimas métricas.
#[must_use = "sin `shutdown` se pierden las métricas del último intervalo"]
pub struct Telemetry {
    meter_provider: Option<SdkMeterProvider>,
}

/// Inicia logs y métricas. Debe llamarse dentro del runtime de tokio (el exportador gRPC lo
/// necesita) y antes de crear cualquier instrumento.
pub fn init(service_name: &'static str) -> Telemetry {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        // Colores solo en una terminal: en `kubectl logs` serían códigos ANSI sueltos.
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    if !ENDPOINT_VARS.iter().any(|var| env::var_os(var).is_some()) {
        info!("métricas desactivadas: falta OTEL_EXPORTER_OTLP_ENDPOINT");
        return Telemetry {
            meter_provider: None,
        };
    }

    let exporter = match MetricExporter::builder().with_tonic().build() {
        Ok(exporter) => exporter,
        Err(error) => {
            // Sin métricas el servicio sigue siendo útil: no se aborta el arranque.
            warn!(%error, "no se pudo crear el exportador OTLP; métricas desactivadas");
            return Telemetry {
                meter_provider: None,
            };
        }
    };
    let provider = SdkMeterProvider::builder()
        .with_resource(Resource::builder().with_service_name(service_name).build())
        // Respeta `OTEL_METRIC_EXPORT_INTERVAL`.
        .with_periodic_exporter(exporter)
        .build();
    global::set_meter_provider(provider.clone());
    info!("métricas OTLP activadas");
    Telemetry {
        meter_provider: Some(provider),
    }
}

impl Telemetry {
    /// Exporta lo pendiente y detiene la exportación.
    pub fn shutdown(self) {
        if let Some(provider) = self.meter_provider
            && let Err(error) = provider.shutdown()
        {
            warn!(%error, "no se pudieron enviar las últimas métricas");
        }
    }
}
