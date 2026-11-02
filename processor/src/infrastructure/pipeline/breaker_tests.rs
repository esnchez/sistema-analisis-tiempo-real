use super::*;

fn breaker() -> CircuitBreaker {
    CircuitBreaker::new(BreakerConfig {
        failure_threshold: 2,
        cooldown: Duration::from_secs(30),
    })
}

#[tokio::test(start_paused = true)]
async fn opens_after_threshold_consecutive_failures() {
    let breaker = breaker();

    breaker.record_failure();
    assert!(!breaker.is_open());
    breaker.record_failure();
    assert!(breaker.is_open());
}

#[tokio::test(start_paused = true)]
async fn success_resets_the_failure_count() {
    let breaker = breaker();

    breaker.record_failure();
    breaker.record_success();
    breaker.record_failure();

    assert!(!breaker.is_open());
}

#[tokio::test(start_paused = true)]
async fn waits_for_cooldown_then_lets_work_through() {
    let breaker = breaker();
    breaker.record_failure();
    breaker.record_failure();
    let start = Instant::now();

    breaker.wait_until_closed().await;

    assert_eq!(start.elapsed(), Duration::from_secs(30));
    assert!(!breaker.is_open());
}

#[tokio::test(start_paused = true)]
async fn half_open_failure_reopens_immediately() {
    let breaker = breaker();
    breaker.record_failure();
    breaker.record_failure();
    breaker.wait_until_closed().await;

    breaker.record_failure();

    assert!(breaker.is_open());
}
