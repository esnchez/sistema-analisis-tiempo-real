//! Reintentos en proceso con backoff exponencial y jitter.

use std::time::Duration;

use tokio::time::sleep;

use crate::application::port::RepositoryError;

/// Política de reintentos para fallos transitorios del almacén.
///
/// Reintentar en proceso mantiene el lote idéntico, así que el almacén deduplica el
/// reintento si la escritura anterior llegó a completarse. El tiempo total debe quedar muy
/// por debajo del `AckWait` del consumidor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Intentos totales, incluido el primero.
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(10),
        }
    }
}

impl RetryPolicy {
    /// Ejecuta `operation` reintentando solo los errores transitorios.
    pub(super) async fn run<F, Fut>(&self, mut operation: F) -> Result<(), RepositoryError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<(), RepositoryError>>,
    {
        let mut attempt = 1;
        loop {
            match operation().await {
                Err(error) if error.is_transient() && attempt < self.max_attempts => {
                    sleep(self.backoff(attempt)).await;
                    attempt += 1;
                }
                result => return result,
            }
        }
    }

    /// Espera tras el intento `attempt` (desde 1): `initial × 2^(attempt-1)`, acotada por
    /// `max_backoff`, con "equal jitter" (entre la mitad y el total) para que los writers no
    /// reintenten a la vez.
    pub(super) fn backoff(&self, attempt: u32) -> Duration {
        let exponential = self
            .initial_backoff
            .saturating_mul(2u32.saturating_pow(attempt.saturating_sub(1)));
        let cap = exponential.min(self.max_backoff);
        cap / 2 + (cap / 2).mul_f64(fastrand::f64())
    }

    /// Peor caso de espera acumulada entre todos los intentos.
    pub fn max_total_backoff(&self) -> Duration {
        (1..self.max_attempts)
            .map(|attempt| {
                self.initial_backoff
                    .saturating_mul(2u32.saturating_pow(attempt - 1))
                    .min(self.max_backoff)
            })
            .sum()
    }
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
