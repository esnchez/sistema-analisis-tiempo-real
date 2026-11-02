//! Comandos (CQS): intenciones de cambiar el estado del sistema. Su manejador no devuelve
//! datos, solo si la operación tuvo éxito.
//!
//! Convención de nombres: el comando es un verbo en imperativo (`RecordEvents`) y su
//! manejador añade el sufijo `Handler` (`RecordEventsHandler`).

mod publish_events;
mod record_events;

pub use publish_events::{PublishEvents, PublishEventsHandler};
pub use record_events::{RecordEvents, RecordEventsHandler};

/// Ejecuta un comando de tipo `C`.
pub trait CommandHandler<C> {
    type Error;

    fn handle(&self, command: C) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
