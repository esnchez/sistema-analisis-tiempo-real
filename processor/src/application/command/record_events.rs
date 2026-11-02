//! Registrar eventos de origen en el almacén analítico.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use super::CommandHandler;
use crate::application::port::{EventRepository, RepositoryError};
use crate::domain::event::SourceEvent;

/// Comando: registrar un lote de eventos ya validados.
///
/// Trabaja por lotes porque el almacén analítico (ClickHouse) rinde con pocas inserciones
/// grandes y se degrada con muchas pequeñas. El consumidor acumula mensajes y emite un
/// comando por lote.
///
/// El lote es inmutable y compartido (`Arc`): reintentar el comando es barato y envía
/// exactamente los mismos eventos, condición para que el almacén deduplique el reintento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordEvents {
    pub events: Arc<[SourceEvent]>,
}

impl RecordEvents {
    pub fn new(events: impl Into<Arc<[SourceEvent]>>) -> Self {
        Self {
            events: events.into(),
        }
    }
}

/// Manejador de [`RecordEvents`]: descarta duplicados del lote y lo anexa al
/// [`EventRepository`].
#[derive(Debug, Clone)]
pub struct RecordEventsHandler<R> {
    repository: R,
}

impl<R: EventRepository> RecordEventsHandler<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R: EventRepository> CommandHandler<RecordEvents> for RecordEventsHandler<R> {
    type Error = RepositoryError;

    async fn handle(&self, command: RecordEvents) -> Result<(), Self::Error> {
        let events = unique_by_id(&command.events);
        if events.is_empty() {
            return Ok(());
        }
        self.repository.append(&events).await
    }
}

/// Elimina eventos con `event_id` repetido (reentregas que caen en el mismo lote),
/// conservando la primera aparición. Solo copia si hay duplicados.
fn unique_by_id(events: &[SourceEvent]) -> Cow<'_, [SourceEvent]> {
    let mut seen = HashSet::with_capacity(events.len());
    if events.iter().all(|event| seen.insert(event.event_id())) {
        return Cow::Borrowed(events);
    }
    seen.clear();
    Cow::Owned(
        events
            .iter()
            .filter(|event| seen.insert(event.event_id()))
            .cloned()
            .collect(),
    )
}

#[cfg(test)]
#[path = "record_events_tests.rs"]
mod tests;
