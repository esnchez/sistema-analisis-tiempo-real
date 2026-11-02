//! Latido para la *liveness probe* del procesador, sin servidor HTTP.
//!
//! Una tarea de tokio escribe la hora Unix (s) en `HEARTBEAT_FILE` cada pocos segundos.
//! Kubernetes comprueba con una probe `exec` que el archivo es reciente:
//!
//! ```sh
//! test $(( $(date +%s) - $(cat /tmp/heartbeat) )) -lt 30
//! ```
//!
//! Demuestra que el proceso y su runtime responden, **no** que haya trabajo: con el broker
//! vacío o el circuit breaker abierto el latido sigue, a propósito. Una caída de NATS o
//! ClickHouse no debe reiniciar los pods (`docs/operations/deployment-and-scaling.md`).
//! El progreso se vigila con métricas (lag del consumidor).

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;

const PERIOD: Duration = Duration::from_secs(5);

/// Lanza el latido si existe `HEARTBEAT_FILE`; si no, no hace nada.
pub fn spawn_from_env() {
    let Some(path) = std::env::var_os("HEARTBEAT_FILE").map(PathBuf::from) else {
        return;
    };
    tokio::spawn(async move {
        let mut ticks = interval(PERIOD);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if let Err(error) = tokio::fs::write(&path, now.to_string()).await {
                warn!(%error, path = %path.display(), "no se pudo escribir el latido");
            }
        }
    });
}
