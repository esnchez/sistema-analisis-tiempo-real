//! Señales del sistema operativo que inician la parada ordenada.

use tracing::info;

/// Vuelve al recibir Ctrl+C o, en Unix, SIGTERM (lo que envía Kubernetes antes de parar
/// un pod).
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate()).expect("no se pudo escuchar SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;

    info!("señal de parada recibida");
}
