//! Procesador de flujos de eventos de usuario para el sistema de análisis en tiempo real.
//!
//! Arquitectura por capas (hexagonal). Las dependencias solo apuntan hacia dentro:
//!
//! - [`domain`]: tipos y reglas del negocio. Sin I/O.
//! - [`application`]: casos de uso y puertos (traits) que necesitan del exterior.
//! - [`infrastructure`]: adaptadores que implementan los puertos (NATS, ClickHouse, Temporal).
//!
//! `main.rs` es la raíz de composición: construye los adaptadores y los inyecta en los casos de uso.

pub mod application;
pub mod domain;
pub mod infrastructure;
