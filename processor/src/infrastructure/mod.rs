//! Capa de infraestructura: adaptadores concretos de los puertos de la aplicación
//! (gateway HTTP, NATS JetStream, ClickHouse), la configuración y la telemetría.

pub mod clickhouse;
mod env;
pub mod heartbeat;
pub mod http;
pub mod nats;
pub mod pipeline;
mod signal;
pub mod telemetry;

pub use env::EnvError;
pub use signal::shutdown_signal;

/// Error opaco de un adaptador (red, protocolo...).
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
