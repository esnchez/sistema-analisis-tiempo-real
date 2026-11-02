//! Ventana de tiempo de evento que se acepta al ingerir.

use chrono::{DateTime, TimeDelta, Utc};

use crate::domain::DomainError;

/// Rango de `occurred_at` aceptado respecto al momento de ingesta.
///
/// Un evento muy antiguo o en el futuro indica un reloj de cliente desajustado o un reenvío
/// muy tardío: no es fiable para el análisis en tiempo real y se aparta para revisarlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventTimeWindow {
    /// Antigüedad máxima respecto a `now`.
    pub max_age: TimeDelta,
    /// Adelanto máximo respecto a `now` (desfase de reloj tolerado).
    pub max_future: TimeDelta,
}

impl Default for EventTimeWindow {
    fn default() -> Self {
        Self {
            max_age: TimeDelta::days(7),
            max_future: TimeDelta::minutes(5),
        }
    }
}

impl EventTimeWindow {
    /// Comprueba que `occurred_at` cae en `[now - max_age, now + max_future]`.
    pub fn check(&self, occurred_at: DateTime<Utc>, now: DateTime<Utc>) -> Result<(), DomainError> {
        let earliest = now - self.max_age;
        let latest = now + self.max_future;
        if (earliest..=latest).contains(&occurred_at) {
            Ok(())
        } else {
            Err(DomainError::OutOfTimeWindow {
                occurred_at,
                earliest,
                latest,
            })
        }
    }
}

#[cfg(test)]
#[path = "time_window_tests.rs"]
mod tests;
