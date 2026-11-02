use std::io;
use std::sync::Mutex;

use chrono::Utc;

use super::*;
use crate::domain::UserId;
use crate::domain::event::{EventId, EventPayload, UserDataEvent};

/// Repositorio en memoria que guarda los `event_id` de cada lote recibido.
#[derive(Default)]
struct FakeRepository {
    batches: Mutex<Vec<Vec<EventId>>>,
    fail: bool,
}

impl EventRepository for FakeRepository {
    async fn append(&self, events: &[SourceEvent]) -> Result<(), RepositoryError> {
        if self.fail {
            return Err(RepositoryError::transient(io::Error::other("caído")));
        }
        let ids = events.iter().map(SourceEvent::event_id).collect();
        self.batches.lock().unwrap().push(ids);
        Ok(())
    }
}

fn event() -> SourceEvent {
    SourceEvent::new(
        UserId::new("u-42").unwrap(),
        Utc::now(),
        None,
        EventPayload::UserData(UserDataEvent::AccountDeleted),
    )
}

fn ids(events: &[SourceEvent]) -> Vec<EventId> {
    events.iter().map(SourceEvent::event_id).collect()
}

#[tokio::test]
async fn appends_whole_batch_in_one_call() {
    let events = vec![event(), event()];
    let expected = ids(&events);
    let handler = RecordEventsHandler::new(FakeRepository::default());

    handler.handle(RecordEvents::new(events)).await.unwrap();

    assert_eq!(*handler.repository.batches.lock().unwrap(), vec![expected]);
}

#[tokio::test]
async fn drops_repeated_event_ids_keeping_first() {
    let (a, b) = (event(), event());
    let handler = RecordEventsHandler::new(FakeRepository::default());

    handler
        .handle(RecordEvents::new(vec![a.clone(), b.clone(), a.clone()]))
        .await
        .unwrap();

    assert_eq!(
        *handler.repository.batches.lock().unwrap(),
        vec![ids(&[a, b])]
    );
}

#[test]
fn unique_by_id_borrows_when_there_are_no_duplicates() {
    let events = [event(), event()];

    assert!(matches!(unique_by_id(&events), Cow::Borrowed(_)));
}

#[tokio::test]
async fn skips_repository_for_empty_batch() {
    let handler = RecordEventsHandler::new(FakeRepository::default());

    handler.handle(RecordEvents::new(Vec::new())).await.unwrap();

    assert!(handler.repository.batches.lock().unwrap().is_empty());
}

#[tokio::test]
async fn propagates_repository_failure() {
    let handler = RecordEventsHandler::new(FakeRepository {
        fail: true,
        ..Default::default()
    });

    let result = handler.handle(RecordEvents::new(vec![event()])).await;

    assert!(result.unwrap_err().is_transient());
}
