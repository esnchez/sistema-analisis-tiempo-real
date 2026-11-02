//! Circuit breaker: deja de pedir mensajes mientras el almacén falla.

use std::sync::Mutex;
use std::time::Duration;

use tokio::time::{Instant, sleep_until};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakerConfig {
    /// Fallos transitorios consecutivos (lotes que agotaron sus reintentos) que abren el circuito.
    pub failure_threshold: u32,
    /// Tiempo con el circuito abierto antes de volver a probar.
    pub cooldown: Duration,
}

impl Default for BreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown: Duration::from_secs(30),
        }
    }
}

/// Estado compartido entre la ingesta (que espera) y los writers (que informan).
///
/// - **Cerrado**: fluye el trabajo.
/// - **Abierto**: la ingesta no pide mensajes hasta que pase `cooldown`. El backlog se queda
///   en el broker, no en memoria, y no se generan reentregas inútiles.
/// - **Semiabierto**: pasado el `cooldown` se reanuda; el primer fallo lo reabre y el primer
///   éxito lo cierra.
#[derive(Debug)]
pub(super) struct CircuitBreaker {
    config: BreakerConfig,
    state: Mutex<State>,
}

#[derive(Debug, Default)]
struct State {
    consecutive_failures: u32,
    open_until: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: BreakerConfig) -> Self {
        Self {
            config,
            state: Mutex::default(),
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        if state.consecutive_failures >= self.config.failure_threshold {
            info!("almacén recuperado: circuito cerrado");
        }
        *state = State::default();
    }

    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        state.consecutive_failures += 1;
        if state.consecutive_failures >= self.config.failure_threshold {
            warn!(
                failures = state.consecutive_failures,
                cooldown = ?self.config.cooldown,
                "circuito abierto: se deja de consumir"
            );
            state.open_until = Some(Instant::now() + self.config.cooldown);
        }
    }

    /// Abierto ahora mismo (semiabierto cuenta como cerrado).
    pub fn is_open(&self) -> bool {
        self.open_until()
            .is_some_and(|until| until > Instant::now())
    }

    /// Vuelve en cuanto el circuito no está abierto.
    pub async fn wait_until_closed(&self) {
        while let Some(until) = self.open_until().filter(|until| *until > Instant::now()) {
            sleep_until(until).await;
        }
    }

    fn open_until(&self) -> Option<Instant> {
        self.state.lock().unwrap().open_until
    }
}

#[cfg(test)]
#[path = "breaker_tests.rs"]
mod tests;
