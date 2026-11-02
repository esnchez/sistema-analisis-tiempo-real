//! Objetos valor: tipos pequeños que validan y normalizan su formato al crearse.

use serde::{Deserialize, Serialize};

use super::DomainError;

/// Exige exactamente `len` letras ASCII y las devuelve en mayúsculas.
fn ascii_letters(
    field: &'static str,
    expected: &'static str,
    len: usize,
    value: String,
) -> Result<String, DomainError> {
    let trimmed = value.trim();
    if trimmed.len() == len && trimmed.bytes().all(|b| b.is_ascii_alphabetic()) {
        Ok(trimmed.to_ascii_uppercase())
    } else {
        Err(DomainError::InvalidFormat { field, expected })
    }
}

validated_string!(
    /// Código de país ISO 3166-1 alfa-2 (p. ej. `ES`). Se normaliza a mayúsculas.
    CountryCode,
    |value| ascii_letters("country", "dos letras ISO 3166-1, p. ej. `ES`", 2, value)
);

validated_string!(
    /// Código de moneda ISO 4217 (p. ej. `EUR`). Se normaliza a mayúsculas.
    Currency,
    |value| ascii_letters("currency", "tres letras ISO 4217, p. ej. `EUR`", 3, value)
);

/// Importe monetario en unidades menores (céntimos) para evitar errores de coma flotante.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    /// Importe en la unidad menor de la moneda (p. ej. `999` = 9,99 EUR).
    pub amount_minor: u64,
    pub currency: Currency,
}

/// Porcentaje entero entre 0 y 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Percent(u8);

impl Percent {
    pub const MAX: u8 = 100;

    pub fn new(value: u8) -> Result<Self, DomainError> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(DomainError::OutOfRange {
                field: "percent",
                min: 0,
                max: Self::MAX.into(),
            })
        }
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Percent {
    type Error = DomainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Percent> for u8 {
    fn from(percent: Percent) -> Self {
        percent.0
    }
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod tests;
