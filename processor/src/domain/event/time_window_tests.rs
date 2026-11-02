use chrono::TimeZone;

use super::*;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap()
}

#[test]
fn accepts_bounds_inclusive() {
    let window = EventTimeWindow::default();

    assert!(window.check(now() - TimeDelta::days(7), now()).is_ok());
    assert!(window.check(now(), now()).is_ok());
    assert!(window.check(now() + TimeDelta::minutes(5), now()).is_ok());
}

#[test]
fn rejects_too_old() {
    let occurred_at = now() - TimeDelta::days(7) - TimeDelta::seconds(1);

    assert!(matches!(
        EventTimeWindow::default().check(occurred_at, now()),
        Err(DomainError::OutOfTimeWindow { .. })
    ));
}

#[test]
fn rejects_too_far_in_the_future() {
    let occurred_at = now() + TimeDelta::minutes(5) + TimeDelta::seconds(1);

    assert!(
        EventTimeWindow::default()
            .check(occurred_at, now())
            .is_err()
    );
}
