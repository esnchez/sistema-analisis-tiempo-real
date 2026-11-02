//! Contenido editorial (artículos, vídeos, podcasts...) con el que interactúa el usuario.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ContentId;

/// Referencia a una pieza de contenido con los metadatos necesarios para analizarla.
///
/// Los metadatos se copian del CMS en cada evento (desnormalizados) para que ClickHouse
/// pueda agregar por sección, tipo o autor sin hacer joins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentRef {
    pub id: ContentId,
    pub content_type: ContentType,
    /// Sección editorial (p. ej. `deportes`, `economia`).
    pub section: Option<String>,
    pub author_id: Option<String>,
    /// Fecha de publicación. Permite medir la frescura de lo que se consume.
    pub published_at: Option<DateTime<Utc>>,
    /// El contenido está detrás del muro de pago.
    #[serde(default)]
    pub paywalled: bool,
}

impl ContentRef {
    /// Referencia sin metadatos opcionales.
    pub fn new(id: ContentId, content_type: ContentType) -> Self {
        Self {
            id,
            content_type,
            section: None,
            author_id: None,
            published_at: None,
            paywalled: false,
        }
    }
}

/// Formato del contenido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Article,
    /// Cobertura en directo que se actualiza con entradas sucesivas.
    LiveBlog,
    Video,
    /// Emisión de vídeo en directo; no tiene duración conocida.
    LiveStream,
    Podcast,
    Gallery,
}
