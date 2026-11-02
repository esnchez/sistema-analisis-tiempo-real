//! Implementación de [`EventRepository`] sobre ClickHouse.

use ::clickhouse::Client;
use ::clickhouse::error::Error as ClickHouseError;
use chrono::Utc;

use super::row::EventRow;
use crate::application::port::{EventRepository, RepositoryError};
use crate::domain::event::SourceEvent;

/// Códigos de excepción de ClickHouse que señalan datos inválidos en el lote. Solo estos
/// se consideran [`RepositoryError::Rejected`]: reintentar el mismo lote fallaría igual.
///
/// Cualquier otro fallo (red, sobrecarga, esquema desplegado a medias...) es transitorio:
/// los mensajes se quedan en NATS hasta que el almacén se recupere, sin acabar en la DLQ.
const DATA_ERROR_CODES: &[u32] = &[
    6,   // CANNOT_PARSE_TEXT
    27,  // CANNOT_PARSE_INPUT_ASSERTION_FAILED
    53,  // TYPE_MISMATCH
    69,  // ARGUMENT_OUT_OF_BOUND
    70,  // CANNOT_CONVERT_TYPE
    117, // INCORRECT_DATA
    131, // TOO_LARGE_STRING_SIZE
];

/// Repositorio de eventos que escribe cada lote con un único `INSERT` en la tabla ancha.
///
/// Reintentos seguros en dos niveles:
/// - cada `INSERT` lleva un `insert_deduplication_token` derivado de los `event_id`, así que
///   ClickHouse descarta el reintento de un lote idéntico ya escrito;
/// - la tabla es un `ReplacingMergeTree` cuya clave termina en `event_id`, que colapsa
///   cualquier duplicado restante al fusionar partes.
#[derive(Clone)]
pub struct ClickHouseEventRepository {
    client: Client,
    table: String,
}

impl ClickHouseEventRepository {
    pub const DEFAULT_TABLE: &str = "events";

    pub fn new(client: Client) -> Self {
        Self::with_table(client, Self::DEFAULT_TABLE)
    }

    pub fn with_table(client: Client, table: impl Into<String>) -> Self {
        Self {
            client,
            table: table.into(),
        }
    }
}

impl EventRepository for ClickHouseEventRepository {
    async fn append(&self, events: &[SourceEvent]) -> Result<(), RepositoryError> {
        if events.is_empty() {
            return Ok(());
        }

        let ingested_at = Utc::now();
        let mut insert = self
            .client
            .insert::<EventRow>(&self.table)
            .await
            .map_err(classify)?
            .with_setting("insert_deduplication_token", deduplication_token(events));
        for event in events {
            insert
                .write(&EventRow::from_event(event, ingested_at))
                .await
                .map_err(classify)?;
        }
        insert.end().await.map_err(classify)
    }
}

/// Token determinista del lote: el mismo conjunto de `event_id` produce el mismo token.
///
/// Combina XOR y suma de los UUID (más el tamaño) para que dos lotes distintos colisionen
/// con probabilidad despreciable. Supone que el lote no repite `event_id` (lo garantiza el
/// handler).
fn deduplication_token(events: &[SourceEvent]) -> String {
    let (xor, sum) = events.iter().fold((0u128, 0u128), |(xor, sum), event| {
        let id = event.event_id().as_uuid().as_u128();
        (xor ^ id, sum.wrapping_add(id))
    });
    format!("{xor:032x}{sum:032x}-{}", events.len())
}

/// Traduce un error del cliente a la semántica del puerto.
fn classify(error: ClickHouseError) -> RepositoryError {
    let rejected = match &error {
        ClickHouseError::BadResponse(reason) => {
            exception_code(reason).is_some_and(|code| DATA_ERROR_CODES.contains(&code))
        }
        // La fila no se pudo codificar: el problema está en los datos.
        ClickHouseError::Custom(_) | ClickHouseError::InvalidUtf8Encoding(_) => true,
        _ => false,
    };
    if rejected {
        RepositoryError::rejected(error)
    } else {
        RepositoryError::transient(error)
    }
}

/// Extrae `<n>` de un mensaje con el formato `Code: <n>. DB::Exception: ...`.
fn exception_code(reason: &str) -> Option<u32> {
    let rest = &reason[reason.find("Code: ")? + "Code: ".len()..];
    let digits = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..digits].parse().ok()
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
