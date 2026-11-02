use std::io;
use std::sync::Mutex;

use super::*;

#[derive(Default)]
struct FakePublisher {
    batches: Mutex<Vec<Vec<RawEvent>>>,
    fail: bool,
}

impl EventPublisher for FakePublisher {
    async fn publish(&self, events: Vec<RawEvent>) -> Result<(), PublishError> {
        if self.fail {
            return Err(PublishError::new(io::Error::other("sin bus")));
        }
        self.batches.lock().unwrap().push(events);
        Ok(())
    }
}

fn raw(payload: &str) -> RawEvent {
    RawEvent {
        payload: payload.as_bytes().to_vec(),
        event_id: None,
        event_type: None,
    }
}

#[tokio::test]
async fn forwards_events_unchanged() {
    let events = vec![raw(r#"{"anything": true}"#), raw("42")];
    let handler = PublishEventsHandler::new(FakePublisher::default());

    handler
        .handle(PublishEvents::new(events.clone()))
        .await
        .unwrap();

    assert_eq!(*handler.publisher.batches.lock().unwrap(), vec![events]);
}

#[tokio::test]
async fn skips_publisher_for_empty_batch() {
    let handler = PublishEventsHandler::new(FakePublisher::default());

    handler
        .handle(PublishEvents::new(Vec::new()))
        .await
        .unwrap();

    assert!(handler.publisher.batches.lock().unwrap().is_empty());
}

#[tokio::test]
async fn propagates_publisher_failure() {
    let handler = PublishEventsHandler::new(FakePublisher {
        fail: true,
        ..Default::default()
    });

    assert!(
        handler
            .handle(PublishEvents::new(vec![raw("{}")]))
            .await
            .is_err()
    );
}
