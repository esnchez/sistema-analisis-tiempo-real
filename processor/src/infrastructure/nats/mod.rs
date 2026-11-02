//! Adaptador de NATS JetStream: topología (streams y consumidor), publicación de eventos de
//! clientes, flujo de mensajes para el [pipeline](crate::infrastructure::pipeline) y DLQ.

mod config;
mod dead_letter;
mod publisher;
mod source;
mod topology;

pub use config::NatsConfig;
pub use dead_letter::NatsDeadLetterQueue;
pub use publisher::NatsEventPublisher;
pub use source::{JetStreamDelivery, deliveries};
pub use topology::{connect, ensure_consumer, ensure_streams};
