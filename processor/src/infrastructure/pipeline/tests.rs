//! Pruebas del pipeline completo con un broker, una DLQ y un handler en memoria.

use std::io;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{TimeDelta, Utc};
use futures_util::{StreamExt, stream};
use tokio::time::timeout;

use super::*;
use crate::domain::UserId;
use crate::domain::event::{EventPayload, SourceEvent, UserDataEvent};

/// Qué le pasó a cada mensaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Outcome {
    Acked,
    Terminated,
}

type Log = Arc<Mutex<Vec<(String, Outcome)>>>;

struct FakeDelivery {
    subject: String,
    payload: Vec<u8>,
    log: Log,
}

impl Delivery for FakeDelivery {
    fn subject(&self) -> &str {
        &self.subject
    }

    fn payload(&self) -> &[u8] {
        &self.payload
    }

    async fn ack(&self) -> Result<(), BoxError> {
        self.log
            .lock()
            .unwrap()
            .push((self.subject.clone(), Outcome::Acked));
        Ok(())
    }

    async fn term(&self) -> Result<(), BoxError> {
        self.log
            .lock()
            .unwrap()
            .push((self.subject.clone(), Outcome::Terminated));
        Ok(())
    }
}

#[derive(Default)]
struct FakeDlq {
    sent: Mutex<Vec<(String, &'static str)>>,
}

impl DeadLetterQueue for FakeDlq {
    async fn send<D: Delivery>(
        &self,
        delivery: &D,
        reason: &DeadLetterReason,
    ) -> Result<(), BoxError> {
        self.sent
            .lock()
            .unwrap()
            .push((delivery.subject().to_owned(), reason.kind()));
        Ok(())
    }
}

type Behavior = Box<dyn Fn(&[SourceEvent]) -> Result<(), RepositoryError> + Send + Sync>;

/// Handler que registra el tamaño de cada lote y responde según `behavior`.
struct FakeHandler {
    behavior: Behavior,
    batch_sizes: Mutex<Vec<usize>>,
}

impl FakeHandler {
    fn new(
        behavior: impl Fn(&[SourceEvent]) -> Result<(), RepositoryError> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            behavior: Box::new(behavior),
            batch_sizes: Mutex::default(),
        })
    }

    fn ok() -> Arc<Self> {
        Self::new(|_| Ok(()))
    }

    fn batch_sizes(&self) -> Vec<usize> {
        self.batch_sizes.lock().unwrap().clone()
    }
}

impl CommandHandler<RecordEvents> for FakeHandler {
    type Error = RepositoryError;

    async fn handle(&self, command: RecordEvents) -> Result<(), RepositoryError> {
        self.batch_sizes.lock().unwrap().push(command.events.len());
        (self.behavior)(&command.events)
    }
}

/// Fábrica de mensajes que comparten el mismo registro de resultados.
#[derive(Default)]
struct Broker {
    log: Log,
}

impl Broker {
    fn event(&self, subject: &str, user: &str, occurred_at: chrono::DateTime<Utc>) -> FakeDelivery {
        let event = SourceEvent::new(
            UserId::new(user).unwrap(),
            occurred_at,
            None,
            EventPayload::UserData(UserDataEvent::AccountDeleted),
        );
        self.raw(subject, serde_json::to_vec(&event).unwrap())
    }

    fn valid(&self, subject: &str) -> FakeDelivery {
        self.event(subject, "u-1", Utc::now())
    }

    fn raw(&self, subject: &str, payload: impl Into<Vec<u8>>) -> FakeDelivery {
        FakeDelivery {
            subject: subject.to_owned(),
            payload: payload.into(),
            log: self.log.clone(),
        }
    }

    fn outcomes(&self) -> Vec<(String, Outcome)> {
        let mut outcomes = self.log.lock().unwrap().clone();
        outcomes.sort();
        outcomes
    }
}

fn acked(subjects: &[&str]) -> Vec<(String, Outcome)> {
    subjects
        .iter()
        .map(|subject| (subject.to_string(), Outcome::Acked))
        .collect()
}

fn config(max_rows: usize) -> PipelineConfig {
    PipelineConfig {
        batch: BatchLimits {
            max_rows,
            max_bytes: usize::MAX,
            max_age: Duration::from_secs(3600),
        },
        retry: RetryPolicy {
            max_attempts: 2,
            initial_backoff: Duration::from_millis(1),
            max_backoff: Duration::from_millis(1),
        },
        ..PipelineConfig::default()
    }
}

/// Ejecuta el pipeline hasta agotar los mensajes.
async fn run_to_end(
    deliveries: Vec<FakeDelivery>,
    handler: Arc<FakeHandler>,
    dlq: Arc<FakeDlq>,
    config: PipelineConfig,
) {
    let run = run(
        stream::iter(deliveries),
        handler,
        dlq,
        config,
        CancellationToken::new(),
    );
    timeout(Duration::from_secs(5), run)
        .await
        .expect("el pipeline no terminó");
}

#[tokio::test]
async fn acks_every_message_after_its_batch_is_persisted() {
    let broker = Broker::default();
    let deliveries = ["m1", "m2", "m3", "m4", "m5"]
        .map(|s| broker.valid(s))
        .into();
    let handler = FakeHandler::ok();

    run_to_end(deliveries, handler.clone(), Arc::default(), config(2)).await;

    let mut sizes = handler.batch_sizes();
    sizes.sort();
    assert_eq!(sizes, vec![1, 2, 2]);
    assert_eq!(broker.outcomes(), acked(&["m1", "m2", "m3", "m4", "m5"]));
}

#[tokio::test]
async fn invalid_and_out_of_window_messages_go_to_dlq_without_blocking_the_batch() {
    let broker = Broker::default();
    let deliveries = vec![
        broker.valid("m1"),
        broker.raw("bad-json", "{ no es json"),
        broker.event("too-old", "u-1", Utc::now() - TimeDelta::days(30)),
        broker.valid("m2"),
    ];
    let handler = FakeHandler::ok();
    let dlq = Arc::new(FakeDlq::default());

    run_to_end(deliveries, handler.clone(), dlq.clone(), config(10)).await;

    assert_eq!(handler.batch_sizes(), vec![2]);
    assert_eq!(
        *dlq.sent.lock().unwrap(),
        vec![
            ("bad-json".to_owned(), "invalid_event"),
            ("too-old".to_owned(), "out_of_time_window"),
        ]
    );
    let mut expected = acked(&["m1", "m2"]);
    expected.push(("bad-json".to_owned(), Outcome::Terminated));
    expected.push(("too-old".to_owned(), Outcome::Terminated));
    expected.sort();
    assert_eq!(broker.outcomes(), expected);
}

#[tokio::test]
async fn rejected_batch_is_bisected_until_the_culprit_is_isolated() {
    let broker = Broker::default();
    let deliveries = vec![
        broker.valid("m1"),
        broker.valid("m2"),
        broker.event("culprit", "u-bad", Utc::now()),
        broker.valid("m3"),
    ];
    // Rechaza cualquier lote que contenga al usuario `u-bad`.
    let handler = FakeHandler::new(|events| {
        if events
            .iter()
            .any(|event| event.user_id().as_str() == "u-bad")
        {
            Err(RepositoryError::rejected(io::Error::other("TYPE_MISMATCH")))
        } else {
            Ok(())
        }
    });
    let dlq = Arc::new(FakeDlq::default());

    run_to_end(deliveries, handler.clone(), dlq.clone(), config(10)).await;

    // [4] ✗ → [2] ✓ + [2] ✗ → [1] ✗ (DLQ) + [1] ✓
    assert_eq!(handler.batch_sizes(), vec![4, 2, 2, 1, 1]);
    assert_eq!(
        *dlq.sent.lock().unwrap(),
        vec![("culprit".to_owned(), "rejected_by_store")]
    );
    let mut expected = acked(&["m1", "m2", "m3"]);
    expected.push(("culprit".to_owned(), Outcome::Terminated));
    expected.sort();
    assert_eq!(broker.outcomes(), expected);
}

#[tokio::test]
async fn transient_failure_leaves_messages_unacked_for_redelivery() {
    let broker = Broker::default();
    let deliveries = vec![broker.valid("m1"), broker.valid("m2")];
    let handler = FakeHandler::new(|_| Err(RepositoryError::transient(io::Error::other("caído"))));
    let dlq = Arc::new(FakeDlq::default());

    run_to_end(deliveries, handler.clone(), dlq.clone(), config(10)).await;

    // 2 intentos (política de reintentos) y nada confirmado ni apartado.
    assert_eq!(handler.batch_sizes(), vec![2, 2]);
    assert!(broker.outcomes().is_empty());
    assert!(dlq.sent.lock().unwrap().is_empty());
}

#[tokio::test]
async fn closes_batch_by_age_while_the_source_stays_open() {
    let broker = Broker::default();
    let source = stream::iter(vec![broker.valid("m1")]).chain(stream::pending());
    let handler = FakeHandler::ok();
    let shutdown = CancellationToken::new();
    let mut config = config(100);
    config.batch.max_age = Duration::from_millis(50);
    let pipeline = tokio::spawn(run(
        source,
        handler.clone(),
        Arc::new(FakeDlq::default()),
        config,
        shutdown.clone(),
    ));

    timeout(Duration::from_secs(5), async {
        while broker.outcomes().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("el lote no se cerró por antigüedad");

    assert_eq!(broker.outcomes(), acked(&["m1"]));
    shutdown.cancel();
    pipeline.await.unwrap();
}

#[tokio::test]
async fn shutdown_flushes_the_open_batch_before_returning() {
    let broker = Broker::default();
    let source = stream::iter(["m1", "m2", "m3"].map(|s| broker.valid(s))).chain(stream::pending());
    let handler = FakeHandler::ok();
    let shutdown = CancellationToken::new();
    // Ni el tamaño ni la edad cerrarían el lote: solo la parada.
    let pipeline = tokio::spawn(run(
        source,
        handler.clone(),
        Arc::new(FakeDlq::default()),
        config(100),
        shutdown.clone(),
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(broker.outcomes().is_empty());

    shutdown.cancel();
    timeout(Duration::from_secs(5), pipeline)
        .await
        .expect("la parada no terminó")
        .unwrap();

    assert_eq!(handler.batch_sizes(), vec![3]);
    assert_eq!(broker.outcomes(), acked(&["m1", "m2", "m3"]));
}

#[test]
fn hold_time_covers_batch_age_and_retries() {
    let config = PipelineConfig::default();

    assert_eq!(
        config.max_hold_time(),
        config.batch.max_age + config.retry.max_total_backoff()
    );
}
