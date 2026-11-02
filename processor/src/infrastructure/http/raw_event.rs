//! Lectura tolerante de los eventos que envían los clientes.

use serde::Deserialize;
use serde_json::Value;
use serde_json::value::RawValue;

use crate::application::port::RawEvent;

/// Campos que se intentan leer para enrutar y deduplicar. Cualquier otro se ignora.
#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    event_id: Value,
    #[serde(default)]
    source: Value,
    #[serde(default)]
    event: Value,
}

/// Convierte un elemento del cuerpo en [`RawEvent`] sin rechazarlo nunca.
///
/// El JSON original se conserva byte a byte. `event_id` y el tipo (`<source>.<event.type>`)
/// solo se rellenan si existen y son cadenas; si el elemento ni siquiera es un objeto, se
/// publica igual y el procesador lo apartará en la DLQ.
pub(super) fn raw_event(element: &RawValue) -> RawEvent {
    let envelope = serde_json::from_str::<Envelope>(element.get()).ok();
    let field = |value: &Value| value.as_str().map(str::to_owned);
    let (event_id, event_type) = match &envelope {
        Some(envelope) => {
            let source = envelope.source.as_str();
            let kind = envelope.event.get("type").and_then(Value::as_str);
            let event_type = source
                .zip(kind)
                .map(|(source, kind)| format!("{source}.{kind}"));
            (field(&envelope.event_id), event_type)
        }
        None => (None, None),
    };
    RawEvent {
        payload: element.get().as_bytes().to_vec(),
        event_id,
        event_type,
    }
}

#[cfg(test)]
#[path = "raw_event_tests.rs"]
mod tests;
