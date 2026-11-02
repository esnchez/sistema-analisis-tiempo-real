//! Lectura de configuración desde variables de entorno.

use std::env;
use std::str::FromStr;
use std::time::Duration;

use thiserror::Error;

/// Variable de entorno presente pero con un valor que no se puede interpretar.
#[derive(Debug, Error)]
#[error("la variable `{name}` tiene un valor inválido: `{value}`")]
pub struct EnvError {
    name: String,
    value: String,
}

/// Valor de la variable `name`, o `default` si no existe.
pub(crate) fn string_or(name: &str, default: impl Into<String>) -> String {
    env::var(name).unwrap_or_else(|_| default.into())
}

/// Valor de la variable `name` interpretado como `T`, o `default` si no existe.
pub(crate) fn parse_or<T: FromStr>(name: &str, default: T) -> Result<T, EnvError> {
    match env::var(name) {
        Ok(value) => value.trim().parse().map_err(|_| EnvError {
            name: name.to_owned(),
            value,
        }),
        Err(_) => Ok(default),
    }
}

/// Duración en milisegundos desde la variable `name`, o `default` si no existe.
pub(crate) fn millis_or(name: &str, default: Duration) -> Result<Duration, EnvError> {
    let millis = parse_or(name, u64::try_from(default.as_millis()).unwrap_or(u64::MAX))?;
    Ok(Duration::from_millis(millis))
}
