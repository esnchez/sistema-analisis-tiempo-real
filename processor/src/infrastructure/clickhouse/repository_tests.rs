use ::clickhouse::test::{Mock, handlers, status};

use super::*;
use crate::domain::UserId;
use crate::domain::event::{EventPayload, UserDataEvent};

fn event() -> SourceEvent {
    SourceEvent::new(
        UserId::new("u-42").unwrap(),
        Utc::now(),
        None,
        EventPayload::UserData(UserDataEvent::AccountDeleted),
    )
}

fn repository(mock: &Mock) -> ClickHouseEventRepository {
    ClickHouseEventRepository::new(Client::default().with_mock(mock))
}

#[tokio::test]
async fn inserts_one_row_per_event() {
    let mock = Mock::new();
    let recording = mock.add(handlers::record::<EventRow>());
    let events = vec![event(), event()];

    repository(&mock).append(&events).await.unwrap();

    let rows: Vec<EventRow> = recording.collect().await;
    let ids: Vec<_> = rows.iter().map(|row| row.event_id).collect();
    let expected: Vec<_> = events.iter().map(|e| *e.event_id().as_uuid()).collect();
    assert_eq!(ids, expected);
    // Todo el lote comparte la misma versión para `ReplacingMergeTree`.
    assert_eq!(rows[0].ingested_at, rows[1].ingested_at);
}

#[tokio::test]
async fn http_failure_is_transient() {
    let mock = Mock::new();
    mock.add(handlers::failure(status::INTERNAL_SERVER_ERROR));

    let error = repository(&mock).append(&[event()]).await.unwrap_err();

    assert!(error.is_transient());
}

#[tokio::test]
async fn data_exception_is_rejected() {
    let mock = Mock::new();
    mock.add(handlers::exception(53)); // TYPE_MISMATCH

    let error = repository(&mock).append(&[event()]).await.unwrap_err();

    assert!(!error.is_transient());
}

#[tokio::test]
async fn overload_exception_is_transient() {
    let mock = Mock::new();
    mock.add(handlers::exception(252)); // TOO_MANY_PARTS

    let error = repository(&mock).append(&[event()]).await.unwrap_err();

    assert!(error.is_transient());
}

#[test]
fn parses_exception_code() {
    assert_eq!(
        exception_code("Code: 252. DB::Exception: Too many parts"),
        Some(252)
    );
    assert_eq!(exception_code("Code: 53"), Some(53));
    assert_eq!(exception_code("Internal Server Error"), None);
}

#[test]
fn token_depends_only_on_the_set_of_event_ids() {
    let (a, b, c) = (event(), event(), event());

    let token = deduplication_token(&[a.clone(), b.clone()]);

    assert_eq!(token, deduplication_token(&[b.clone(), a.clone()]));
    assert_ne!(token, deduplication_token(&[a.clone(), c]));
    assert_ne!(token, deduplication_token(&[a]));
}
