//! Fila de la tabla ancha `events`: un [`SourceEvent`] aplanado y desnormalizado.
//!
//! Debe coincidir columna a columna con `config/clickhouse.sql`. Los campos que no aplican
//! al tipo de evento quedan con su valor por defecto.

use ::clickhouse::Row;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::event::{EventPayload, SourceEvent, UserBehaviorEvent, UserDataEvent};
use crate::domain::{ClientContext, ContentRef, CountryCode, Money};

#[derive(Debug, Clone, Default, PartialEq, Eq, Row, Serialize, Deserialize)]
pub(super) struct EventRow {
    // Identidad y tiempo
    #[serde(with = "::clickhouse::serde::uuid")]
    pub event_id: Uuid,
    pub event_type: String,
    pub user_id: String,
    #[serde(with = "::clickhouse::serde::chrono::datetime64::millis")]
    pub occurred_at: DateTime<Utc>,
    #[serde(with = "::clickhouse::serde::chrono::datetime64::millis")]
    pub ingested_at: DateTime<Utc>,

    // Contexto del cliente
    pub session_id: String,
    pub authenticated: bool,
    pub platform: String,
    pub device_type: String,
    pub app_version: String,
    pub country: String,

    // Contenido
    pub content_id: String,
    pub content_type: String,
    pub content_section: String,
    pub content_author_id: String,
    #[serde(with = "::clickhouse::serde::chrono::datetime64::millis::option")]
    pub content_published_at: Option<DateTime<Utc>>,
    pub content_paywalled: bool,

    // Adquisición
    pub traffic_source: String,
    pub referrer: String,
    pub campaign: String,

    // Lectura
    pub engaged_secs: u32,
    pub scroll_depth: u8,

    // Reproducción
    pub playback_action: String,
    pub position_secs: u32,
    pub duration_secs: Option<u32>,
    pub bitrate_kbps: Option<u32>,

    // Compartir, búsqueda y recomendaciones
    pub share_channel: String,
    pub search_query: String,
    pub search_results_count: u32,
    pub recommendation_module: String,
    pub recommended_content_ids: Vec<String>,
    pub recommendation_position: u16,

    // Publicidad
    pub ad_id: String,
    pub ad_placement: String,
    pub ad_action: String,

    // Perfil
    pub signup_method: String,
    pub profile_country: String,
    pub language: String,
    pub age_range: String,
    pub interests: Vec<String>,

    // Suscripción
    pub subscription_change: String,
    pub subscription_plan: String,
    pub previous_subscription_plan: String,
    pub billing_period: String,
    pub price_amount_minor: Option<u64>,
    pub price_currency: String,
    pub trial: bool,

    // Consentimientos
    pub consent_analytics: bool,
    pub consent_personalization: bool,
    pub consent_advertising: bool,

    // Newsletters
    pub newsletter: String,
    pub newsletter_subscribed: bool,
}

impl EventRow {
    /// Aplana el evento. `ingested_at` es la versión de la fila para `ReplacingMergeTree`.
    pub fn from_event(event: &SourceEvent, ingested_at: DateTime<Utc>) -> Self {
        let mut row = Self {
            event_id: *event.event_id().as_uuid(),
            event_type: event.event_type().to_owned(),
            user_id: event.user_id().to_string(),
            occurred_at: event.occurred_at(),
            ingested_at,
            ..Self::default()
        };
        if let Some(context) = event.context() {
            row.set_context(context);
        }
        match event.payload() {
            EventPayload::UserBehavior(behavior) => row.set_user_behavior(behavior),
            EventPayload::UserData(data) => row.set_user_data(data),
        }
        row
    }

    fn set_context(&mut self, context: &ClientContext) {
        self.session_id = text(context.session_id.as_ref());
        self.authenticated = context.authenticated;
        self.platform = label(&context.platform);
        self.device_type = label(&context.device_type);
        self.app_version = context.app_version.clone().unwrap_or_default();
        self.country = text(context.country.as_ref());
    }

    fn set_content(&mut self, content: &ContentRef) {
        self.content_id = content.id.to_string();
        self.content_type = label(&content.content_type);
        self.content_section = content.section.clone().unwrap_or_default();
        self.content_author_id = content.author_id.clone().unwrap_or_default();
        self.content_published_at = content.published_at;
        self.content_paywalled = content.paywalled;
    }

    fn set_user_behavior(&mut self, event: &UserBehaviorEvent) {
        if let Some(content) = event.content() {
            self.set_content(content);
        }
        match event {
            UserBehaviorEvent::ContentViewed {
                traffic_source,
                referrer,
                campaign,
                ..
            } => {
                self.traffic_source = label(traffic_source);
                self.referrer = referrer.clone().unwrap_or_default();
                self.campaign = campaign.clone().unwrap_or_default();
            }
            UserBehaviorEvent::ContentEngaged {
                engaged_secs,
                scroll_depth,
                ..
            } => {
                self.engaged_secs = *engaged_secs;
                self.scroll_depth = scroll_depth.value();
            }
            UserBehaviorEvent::PlaybackStateChanged {
                action,
                position_secs,
                duration_secs,
                bitrate_kbps,
                ..
            } => {
                self.playback_action = label(action);
                self.position_secs = *position_secs;
                self.duration_secs = *duration_secs;
                self.bitrate_kbps = *bitrate_kbps;
            }
            UserBehaviorEvent::ContentShared { channel, .. } => {
                self.share_channel = label(channel);
            }
            UserBehaviorEvent::SearchPerformed {
                query,
                results_count,
            } => {
                self.search_query = query.clone();
                self.search_results_count = *results_count;
            }
            UserBehaviorEvent::RecommendationShown {
                module,
                content_ids,
            } => {
                self.recommendation_module = module.clone();
                self.recommended_content_ids =
                    content_ids.iter().map(|id| id.to_string()).collect();
            }
            UserBehaviorEvent::RecommendationClicked {
                module, position, ..
            } => {
                self.recommendation_module = module.clone();
                self.recommendation_position = *position;
            }
            UserBehaviorEvent::PaywallShown { .. } => {}
            UserBehaviorEvent::AdInteracted {
                ad_id,
                placement,
                action,
                content_id,
            } => {
                self.ad_id = ad_id.clone();
                self.ad_placement = label(placement);
                self.ad_action = label(action);
                self.content_id = text(content_id.as_ref());
            }
        }
    }

    fn set_user_data(&mut self, event: &UserDataEvent) {
        match event {
            UserDataEvent::Registered {
                signup_method,
                country,
                language,
                age_range,
                traffic_source,
            } => {
                self.signup_method = label(signup_method);
                self.set_profile(country.as_ref(), language.as_ref(), age_range.as_ref());
                self.traffic_source = traffic_source.as_ref().map(label).unwrap_or_default();
            }
            UserDataEvent::ProfileUpdated {
                country,
                language,
                age_range,
                interests,
            } => {
                self.set_profile(country.as_ref(), language.as_ref(), age_range.as_ref());
                self.interests = interests.clone().unwrap_or_default();
            }
            UserDataEvent::SubscriptionChanged {
                change,
                plan,
                previous_plan,
                billing_period,
                price,
                trial,
            } => {
                self.subscription_change = label(change);
                self.subscription_plan = label(plan);
                self.previous_subscription_plan =
                    previous_plan.as_ref().map(label).unwrap_or_default();
                self.billing_period = billing_period.as_ref().map(label).unwrap_or_default();
                if let Some(Money {
                    amount_minor,
                    currency,
                }) = price
                {
                    self.price_amount_minor = Some(*amount_minor);
                    self.price_currency = currency.to_string();
                }
                self.trial = *trial;
            }
            UserDataEvent::ConsentUpdated {
                analytics,
                personalization,
                advertising,
            } => {
                self.consent_analytics = *analytics;
                self.consent_personalization = *personalization;
                self.consent_advertising = *advertising;
            }
            UserDataEvent::NewsletterSubscriptionChanged {
                newsletter,
                subscribed,
            } => {
                self.newsletter = newsletter.clone();
                self.newsletter_subscribed = *subscribed;
            }
            UserDataEvent::AccountDeleted => {}
        }
    }

    fn set_profile<A: Serialize>(
        &mut self,
        country: Option<&CountryCode>,
        language: Option<&String>,
        age_range: Option<&A>,
    ) {
        self.profile_country = text(country);
        self.language = language.cloned().unwrap_or_default();
        self.age_range = age_range.map(label).unwrap_or_default();
    }
}

/// Texto de un valor opcional; cadena vacía si no hay valor.
fn text(value: Option<&impl ToString>) -> String {
    value.map(ToString::to_string).unwrap_or_default()
}

/// Nombre con el que serde serializa una variante unitaria (p. ej. `PlaybackAction::BufferingStarted`
/// → `buffering_started`). Así lo almacenado coincide con el formato JSON de las fuentes.
fn label<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        other => unreachable!("se esperaba una variante unitaria, se obtuvo {other:?}"),
    }
}

#[cfg(test)]
#[path = "row_tests.rs"]
mod tests;
