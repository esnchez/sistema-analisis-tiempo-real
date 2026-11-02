//! Conexión a ClickHouse.

use ::clickhouse::Client;

use crate::infrastructure::env::string_or;

/// Parámetros de conexión por HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickHouseConfig {
    pub url: String,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl Default for ClickHouseConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8123".into(),
            database: "default".into(),
            user: "default".into(),
            password: String::new(),
        }
    }
}

impl ClickHouseConfig {
    /// Lee `CLICKHOUSE_URL`, `CLICKHOUSE_DATABASE`, `CLICKHOUSE_USER` y `CLICKHOUSE_PASSWORD`.
    /// Las que falten toman el valor de [`Default`].
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            url: string_or("CLICKHOUSE_URL", default.url),
            database: string_or("CLICKHOUSE_DATABASE", default.database),
            user: string_or("CLICKHOUSE_USER", default.user),
            password: string_or("CLICKHOUSE_PASSWORD", default.password),
        }
    }

    /// Crea el cliente. No abre conexiones hasta la primera consulta.
    pub fn client(&self) -> Client {
        Client::default()
            .with_url(&self.url)
            .with_database(&self.database)
            .with_user(&self.user)
            .with_password(&self.password)
    }
}
