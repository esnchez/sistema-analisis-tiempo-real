//! Publicar en el bus los eventos que envían los clientes.

use super::CommandHandler;
use crate::application::port::{EventPublisher, PublishError, RawEvent};

/// Comando: publicar un lote de eventos de cliente, sin validar su contenido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishEvents {
    pub events: Vec<RawEvent>,
}

impl PublishEvents {
    pub fn new(events: Vec<RawEvent>) -> Self {
        Self { events }
    }
}

/// Manejador de [`PublishEvents`]: entrega el lote al [`EventPublisher`].
///
/// No valida a propósito: la entrada acepta lo que mande el cliente y la validación vive
/// en un único sitio, el procesador, que aparta lo inválido en la DLQ.
#[derive(Debug, Clone)]
pub struct PublishEventsHandler<P> {
    publisher: P,
}

impl<P: EventPublisher> PublishEventsHandler<P> {
    pub fn new(publisher: P) -> Self {
        Self { publisher }
    }
}

impl<P: EventPublisher> CommandHandler<PublishEvents> for PublishEventsHandler<P> {
    type Error = PublishError;

    async fn handle(&self, command: PublishEvents) -> Result<(), Self::Error> {
        if command.events.is_empty() {
            return Ok(());
        }
        self.publisher.publish(command.events).await
    }
}

#[cfg(test)]
#[path = "publish_events_tests.rs"]
mod tests;
