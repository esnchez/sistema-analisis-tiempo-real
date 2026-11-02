//! Errores de validación del dominio.

use chrono::{DateTime, Utc};
use thiserror::Error;

/// Violación de una invariante del dominio.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("el campo `{0}` no puede estar vacío")]
    EmptyField(&'static str),

    #[error("el campo `{field}` supera la longitud máxima de {max} caracteres")]
    TooLong { field: &'static str, max: usize },

    #[error("el campo `{field}` no tiene un formato válido: se esperaba {expected}")]
    InvalidFormat {
        field: &'static str,
        expected: &'static str,
    },

    #[error("el campo `{field}` debe estar entre {min} y {max}")]
    OutOfRange {
        field: &'static str,
        min: u64,
        max: u64,
    },

    #[error(
        "el evento ocurrió en {occurred_at}, fuera de la ventana aceptada [{earliest}, {latest}]"
    )]
    OutOfTimeWindow {
        occurred_at: DateTime<Utc>,
        earliest: DateTime<Utc>,
        latest: DateTime<Utc>,
    },
}
