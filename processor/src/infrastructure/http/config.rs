//! Configuración del gateway HTTP.

use std::net::SocketAddr;

use crate::infrastructure::env::{EnvError, parse_or};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayConfig {
    pub listen_addr: SocketAddr,
    /// Tamaño máximo del cuerpo. No supera el `max_payload` de NATS (1 MiB por defecto),
    /// así ningún evento individual puede superarlo.
    pub max_body_bytes: usize,
    /// Eventos máximos por petición.
    pub max_events: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::from(([0, 0, 0, 0], 8080)),
            max_body_bytes: 1024 * 1024,
            max_events: 500,
        }
    }
}

impl GatewayConfig {
    /// Lee `GATEWAY_ADDR`, `GATEWAY_MAX_BODY_BYTES` y `GATEWAY_MAX_EVENTS`.
    pub fn from_env() -> Result<Self, EnvError> {
        let default = Self::default();
        Ok(Self {
            listen_addr: parse_or("GATEWAY_ADDR", default.listen_addr)?,
            max_body_bytes: parse_or("GATEWAY_MAX_BODY_BYTES", default.max_body_bytes)?,
            max_events: parse_or("GATEWAY_MAX_EVENTS", default.max_events)?,
        })
    }
}
