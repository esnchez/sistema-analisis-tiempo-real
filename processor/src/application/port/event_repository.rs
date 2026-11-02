//! Persistencia de eventos de origen en el almacén analítico.

use std::error::Error;

use thiserror::Error;

use crate::domain::event::SourceEvent;

/// Almacén de eventos de solo anexado.
pub trait EventRepository: Send + Sync {
    /// Persiste el lote completo en una sola escritura.
    ///
    /// Debe ser seguro reintentarlo: un evento escrito dos veces (mismo `event_id`) cuenta
    /// como uno. Así el consumidor puede confirmar los mensajes solo tras persistirlos.
    fn append(
        &self,
        events: &[SourceEvent],
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

/// Fallo del almacén al persistir. El lote no se ha guardado.
///
/// La variante decide qué hace el llamador: reintentar el mismo lote más tarde o aislar
/// los eventos que el almacén no acepta.
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// El almacén no está disponible o no puede aceptar el lote ahora (red, sobrecarga,
    /// esquema desplegado a medias...). Reintentar el mismo lote más tarde.
    #[error("almacén no disponible temporalmente")]
    Transient(#[source] Box<dyn Error + Send + Sync>),

    /// El almacén rechaza el contenido del lote. Reintentarlo igual fallará de nuevo:
    /// hay que localizar los eventos culpables.
    #[error("el almacén rechazó el lote")]
    Rejected(#[source] Box<dyn Error + Send + Sync>),
}

impl RepositoryError {
    pub fn transient(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::Transient(source.into())
    }

    pub fn rejected(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::Rejected(source.into())
    }

    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Transient(_))
    }
}
