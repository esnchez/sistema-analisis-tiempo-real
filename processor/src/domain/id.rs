//! Identificadores opacos emitidos por las fuentes. El sistema no interpreta su contenido.

use super::DomainError;

/// Longitud máxima, en caracteres, de cualquier identificador opaco.
const MAX_LEN: usize = 128;

/// Recorta espacios y exige que el identificador no quede vacío ni supere [`MAX_LEN`].
fn opaque(field: &'static str, value: String) -> Result<String, DomainError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::EmptyField(field));
    }
    if trimmed.chars().count() > MAX_LEN {
        return Err(DomainError::TooLong {
            field,
            max: MAX_LEN,
        });
    }
    Ok(if trimmed.len() == value.len() {
        value
    } else {
        trimmed.to_owned()
    })
}

validated_string!(
    /// Usuario que genera el evento. Si no ha iniciado sesión, es un identificador
    /// anónimo (cookie o dispositivo); ver [`ClientContext::authenticated`](super::ClientContext).
    UserId,
    |value| opaque("user_id", value)
);

validated_string!(
    /// Pieza de contenido en el CMS del medio.
    ContentId,
    |value| opaque("content_id", value)
);

validated_string!(
    /// Sesión de navegación o de app: agrupa los eventos de una misma visita.
    SessionId,
    |value| opaque("session_id", value)
);

#[cfg(test)]
#[path = "id_tests.rs"]
mod tests;
