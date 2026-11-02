//! Contexto del cliente en el que ocurre un evento: sesión, dispositivo y procedencia.

use serde::{Deserialize, Serialize};

use super::{CountryCode, SessionId};

/// Datos del cliente (web, app, smart TV) que emitió el evento.
///
/// Lo envían las fuentes de comportamiento. Las fuentes de datos de usuario suelen ser
/// sistemas internos (CRM, pagos) y no lo incluyen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientContext {
    pub session_id: Option<SessionId>,
    /// El usuario había iniciado sesión. Si es `false`, `user_id` es un identificador anónimo.
    #[serde(default)]
    pub authenticated: bool,
    pub platform: Platform,
    pub device_type: DeviceType,
    /// Versión de la app o del frontend web.
    pub app_version: Option<String>,
    /// País deducido de la IP del cliente.
    pub country: Option<CountryCode>,
}

/// Aplicación o entorno desde el que se accede.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Web,
    Ios,
    Android,
    SmartTv,
}

/// Tipo de dispositivo según su factor de forma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Tv,
}

/// Canal por el que llegó el usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficSource {
    /// URL escrita a mano, marcador o app abierta sin enlace.
    Direct,
    /// Buscadores (Google, Bing...).
    Search,
    /// Redes sociales.
    Social,
    Newsletter,
    PushNotification,
    /// Enlace desde otra página del propio medio.
    Internal,
    /// Enlace desde otro sitio web.
    Referral,
    /// Campañas de pago.
    Paid,
}
