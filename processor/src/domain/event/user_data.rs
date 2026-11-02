//! Eventos de datos de usuario: cambios en quién es el usuario
//! (alta, perfil, suscripción, consentimientos, newsletters, baja).

use serde::{Deserialize, Serialize};

use crate::domain::{CountryCode, Money, TrafficSource};

/// Evento emitido por una fuente de datos de usuario (registro, CRM, pasarela de pago...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserDataEvent {
    /// El usuario creó su cuenta.
    Registered {
        signup_method: SignupMethod,
        country: Option<CountryCode>,
        /// Etiqueta de idioma BCP 47 (p. ej. `es-ES`).
        language: Option<String>,
        age_range: Option<AgeRange>,
        /// Canal de adquisición: por dónde llegó el usuario antes de registrarse.
        traffic_source: Option<TrafficSource>,
    },
    /// El usuario modificó su perfil. Los campos a `None` no cambiaron.
    ProfileUpdated {
        country: Option<CountryCode>,
        language: Option<String>,
        age_range: Option<AgeRange>,
        /// Temas que el usuario declara seguir (p. ej. `futbol`, `tecnologia`).
        interests: Option<Vec<String>>,
    },
    /// Cambió la suscripción del usuario: alta, mejora, renovación, cancelación...
    SubscriptionChanged {
        change: SubscriptionChange,
        /// Plan tras el cambio.
        plan: SubscriptionPlan,
        previous_plan: Option<SubscriptionPlan>,
        billing_period: Option<BillingPeriod>,
        /// Precio por periodo de facturación; `None` en planes gratuitos.
        price: Option<Money>,
        /// La suscripción está en periodo de prueba.
        #[serde(default)]
        trial: bool,
    },
    /// El usuario actualizó los permisos de uso de sus datos.
    /// Sin `personalization` no se debe personalizar contenido para él.
    ConsentUpdated {
        analytics: bool,
        personalization: bool,
        advertising: bool,
    },
    /// El usuario se apuntó o se dio de baja de una newsletter.
    NewsletterSubscriptionChanged {
        newsletter: String,
        subscribed: bool,
    },
    /// El usuario borró su cuenta: sus datos deben anonimizarse (RGPD, derecho de supresión).
    AccountDeleted,
}

impl UserDataEvent {
    /// Nombre estable del tipo de evento, con el prefijo de su fuente.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Registered { .. } => "user_data.registered",
            Self::ProfileUpdated { .. } => "user_data.profile_updated",
            Self::SubscriptionChanged { .. } => "user_data.subscription_changed",
            Self::ConsentUpdated { .. } => "user_data.consent_updated",
            Self::NewsletterSubscriptionChanged { .. } => {
                "user_data.newsletter_subscription_changed"
            }
            Self::AccountDeleted => "user_data.account_deleted",
        }
    }
}

/// Método con el que el usuario creó la cuenta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignupMethod {
    Email,
    Google,
    Apple,
    Facebook,
}

/// Franja de edad declarada por el usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AgeRange {
    #[serde(rename = "under_18")]
    Under18,
    #[serde(rename = "18_24")]
    From18To24,
    #[serde(rename = "25_34")]
    From25To34,
    #[serde(rename = "35_44")]
    From35To44,
    #[serde(rename = "45_54")]
    From45To54,
    #[serde(rename = "55_64")]
    From55To64,
    #[serde(rename = "65_plus")]
    Over65,
}

/// Plan de suscripción.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlan {
    Free,
    Basic,
    Premium,
}

/// Tipo de cambio en la suscripción. Sirve para medir altas, conversión y abandono.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionChange {
    Started,
    Upgraded,
    Downgraded,
    Renewed,
    /// El usuario canceló; puede seguir activo hasta el fin del periodo pagado.
    Cancelled,
    /// Terminó el periodo pagado o de prueba sin renovarse.
    Expired,
}

/// Frecuencia de cobro de la suscripción.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingPeriod {
    Monthly,
    Yearly,
}
