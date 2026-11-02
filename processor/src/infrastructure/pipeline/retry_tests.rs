use std::io;

use super::*;

fn policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 3,
        initial_backoff: Duration::from_secs(1),
        max_backoff: Duration::from_secs(10),
    }
}

#[tokio::test(start_paused = true)]
async fn retries_transient_errors_up_to_max_attempts() {
    let mut calls = 0;

    let result = policy()
        .run(|| {
            calls += 1;
            async { Err(RepositoryError::transient(io::Error::other("caído"))) }
        })
        .await;

    assert!(result.unwrap_err().is_transient());
    assert_eq!(calls, 3);
}

#[tokio::test(start_paused = true)]
async fn stops_retrying_after_success() {
    let mut calls = 0;

    let result = policy()
        .run(|| {
            calls += 1;
            let outcome = if calls < 2 {
                Err(RepositoryError::transient(io::Error::other("caído")))
            } else {
                Ok(())
            };
            async { outcome }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(calls, 2);
}

#[tokio::test(start_paused = true)]
async fn does_not_retry_rejections() {
    let mut calls = 0;

    let result = policy()
        .run(|| {
            calls += 1;
            async { Err(RepositoryError::rejected(io::Error::other("datos"))) }
        })
        .await;

    assert!(!result.unwrap_err().is_transient());
    assert_eq!(calls, 1);
}

#[test]
fn backoff_grows_exponentially_with_jitter_and_cap() {
    let policy = policy();

    for (attempt, cap) in [(1, 1), (2, 2), (3, 4), (5, 10), (30, 10)] {
        let cap = Duration::from_secs(cap);
        let wait = policy.backoff(attempt);
        assert!(
            wait >= cap / 2 && wait <= cap,
            "intento {attempt}: {wait:?}"
        );
    }
}

#[test]
fn max_total_backoff_sums_every_wait() {
    // 3 intentos → esperas tras el 1.º (1 s) y el 2.º (2 s).
    assert_eq!(policy().max_total_backoff(), Duration::from_secs(3));
}
