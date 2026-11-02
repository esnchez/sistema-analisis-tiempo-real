//! Evento de origen: el mensaje que las fuentes publican en NATS y que consume el procesador.
//!
//! Formato JSON:
//!
//! ```json
//! {
//!   "event_id": "0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70",
//!   "user_id": "u-42",
//!   "occurred_at": "2026-10-06T10:15:00Z",
//!   "context": {
//!     "session_id": "s-981",
//!     "authenticated": true,
//!     "platform": "web",
//!     "device_type": "mobile",
//!     "country": "ES"
//!   },
//!   "source": "user_behavior",
//!   "event": {
//!     "type": "content_viewed",
//!     "content": { "id": "article-123", "content_type": "article", "section": "deportes" },
//!     "traffic_source": "social"
//!   }
//! }
//! ```
//!
//! `context` es opcional: las fuentes de datos de usuario (sistemas internos) no lo envían.

mod time_window;
mod user_behavior;
mod user_data;

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ClientContext, UserId};

pub use time_window::EventTimeWindow;
pub use user_behavior::{AdAction, AdPlacement, PlaybackAction, ShareChannel, UserBehaviorEvent};
pub use user_data::{
    AgeRange, BillingPeriod, SignupMethod, SubscriptionChange, SubscriptionPlan, UserDataEvent,
};

/// Identificador único del evento. Permite descartar entregas duplicadas.
///
/// Los nuevos se generan como UUID v7, ordenados por tiempo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Evento recibido de una fuente. Si existe, ya es válido: sus campos se validan
/// al construirlo o deserializarlo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvent {
    event_id: EventId,
    user_id: UserId,
    /// Momento en que ocurrió en la fuente, no cuando llegó al procesador.
    occurred_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context: Option<ClientContext>,
    #[serde(flatten)]
    payload: EventPayload,
}

impl SourceEvent {
    /// Crea un evento con un [`EventId`] nuevo.
    pub fn new(
        user_id: UserId,
        occurred_at: DateTime<Utc>,
        context: Option<ClientContext>,
        payload: EventPayload,
    ) -> Self {
        Self {
            event_id: EventId::new(),
            user_id,
            occurred_at,
            context,
            payload,
        }
    }

    pub fn event_id(&self) -> EventId {
        self.event_id
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    pub fn context(&self) -> Option<&ClientContext> {
        self.context.as_ref()
    }

    pub fn payload(&self) -> &EventPayload {
        &self.payload
    }

    /// Nombre estable del tipo de evento, p. ej. `user_behavior.content_viewed`.
    pub fn event_type(&self) -> &'static str {
        self.payload.name()
    }
}

/// Contenido del evento, según el tipo de fuente que lo emitió.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", content = "event", rename_all = "snake_case")]
pub enum EventPayload {
    /// Datos del usuario: quién es.
    UserData(UserDataEvent),
    /// Comportamiento del usuario: qué hace.
    UserBehavior(UserBehaviorEvent),
}

impl EventPayload {
    /// Nombre estable del tipo de evento, con el prefijo de su fuente.
    pub fn name(&self) -> &'static str {
        match self {
            Self::UserData(event) => event.name(),
            Self::UserBehavior(event) => event.name(),
        }
    }
}

#[cfg(test)]
mod tests;
