//! Eventos de comportamiento: lo que el usuario hace con el contenido
//! (leer, reproducir, compartir, buscar, recomendaciones, muro de pago, publicidad).

use serde::{Deserialize, Serialize};

use crate::domain::{ContentId, ContentRef, Percent, TrafficSource};

/// Evento emitido por una fuente de comportamiento (web, apps, reproductor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserBehaviorEvent {
    /// El usuario abrió una pieza de contenido.
    ContentViewed {
        content: ContentRef,
        traffic_source: TrafficSource,
        /// URL de la página de procedencia, si se conoce.
        referrer: Option<String>,
        /// Campaña de marketing (`utm_campaign`), si la hay.
        campaign: Option<String>,
    },
    /// Latido de lectura: se emite periódicamente (p. ej. cada 15 s) mientras el
    /// usuario está activo en el contenido. Los valores son acumulados.
    ContentEngaged {
        content: ContentRef,
        /// Tiempo activo (con la pestaña visible y con interacción) desde que abrió el contenido.
        engaged_secs: u32,
        /// Máxima profundidad de scroll alcanzada.
        scroll_depth: Percent,
    },
    /// Cambió el estado del reproductor de audio o vídeo.
    PlaybackStateChanged {
        content: ContentRef,
        action: PlaybackAction,
        /// Posición de reproducción, en segundos, en el momento del cambio.
        position_secs: u32,
        /// Duración total en segundos; `None` en directos.
        duration_secs: Option<u32>,
        /// Calidad de la emisión en ese momento, para medir calidad de experiencia.
        bitrate_kbps: Option<u32>,
    },
    /// El usuario compartió un contenido.
    ContentShared {
        content: ContentRef,
        channel: ShareChannel,
    },
    /// El usuario hizo una búsqueda en el sitio.
    SearchPerformed {
        query: String,
        /// Número de resultados; las búsquedas sin resultados señalan huecos de contenido.
        results_count: u32,
    },
    /// Se mostró al usuario un bloque de recomendaciones.
    RecommendationShown {
        /// Bloque que muestra las recomendaciones (p. ej. `related`, `for_you`).
        module: String,
        /// Contenidos recomendados, en el orden mostrado.
        content_ids: Vec<ContentId>,
    },
    /// El usuario pulsó una recomendación. Junto con `RecommendationShown` da el CTR.
    RecommendationClicked {
        module: String,
        content: ContentRef,
        /// Posición del contenido en el bloque, empezando en 0.
        position: u16,
    },
    /// El usuario chocó con el muro de pago. Inicio del embudo de suscripción.
    PaywallShown { content: ContentRef },
    /// Interacción con un anuncio.
    AdInteracted {
        ad_id: String,
        placement: AdPlacement,
        action: AdAction,
        /// Contenido junto al que se mostró el anuncio, si aplica.
        content_id: Option<ContentId>,
    },
}

impl UserBehaviorEvent {
    /// Nombre estable del tipo de evento, con el prefijo de su fuente.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ContentViewed { .. } => "user_behavior.content_viewed",
            Self::ContentEngaged { .. } => "user_behavior.content_engaged",
            Self::PlaybackStateChanged { .. } => "user_behavior.playback_state_changed",
            Self::ContentShared { .. } => "user_behavior.content_shared",
            Self::SearchPerformed { .. } => "user_behavior.search_performed",
            Self::RecommendationShown { .. } => "user_behavior.recommendation_shown",
            Self::RecommendationClicked { .. } => "user_behavior.recommendation_clicked",
            Self::PaywallShown { .. } => "user_behavior.paywall_shown",
            Self::AdInteracted { .. } => "user_behavior.ad_interacted",
        }
    }

    /// Contenido principal al que se refiere el evento, si lo hay.
    pub fn content(&self) -> Option<&ContentRef> {
        match self {
            Self::ContentViewed { content, .. }
            | Self::ContentEngaged { content, .. }
            | Self::PlaybackStateChanged { content, .. }
            | Self::ContentShared { content, .. }
            | Self::RecommendationClicked { content, .. }
            | Self::PaywallShown { content } => Some(content),
            Self::SearchPerformed { .. }
            | Self::RecommendationShown { .. }
            | Self::AdInteracted { .. } => None,
        }
    }
}

/// Acción del reproductor que provocó el evento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackAction {
    Started,
    /// Latido periódico durante la reproducción, para medir tiempo visto.
    Heartbeat,
    Paused,
    Resumed,
    Seeked,
    BufferingStarted,
    BufferingEnded,
    Completed,
    /// La reproducción falló.
    Failed,
}

/// Canal por el que se compartió un contenido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareChannel {
    Facebook,
    X,
    Whatsapp,
    Telegram,
    Linkedin,
    Email,
    CopyLink,
    Other,
}

/// Posición del anuncio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdPlacement {
    /// Antes del vídeo.
    PreRoll,
    /// En mitad del vídeo.
    MidRoll,
    /// Después del vídeo.
    PostRoll,
    /// Banner en la página.
    Display,
    /// Integrado en el contenido con su mismo formato.
    Native,
}

/// Qué hizo el usuario con el anuncio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdAction {
    Impression,
    Click,
    Skipped,
    Completed,
}
