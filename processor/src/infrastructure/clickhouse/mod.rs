//! Adaptador de ClickHouse: persiste los eventos en la tabla ancha `events`
//! (esquema en `config/clickhouse.sql`).

mod config;
mod repository;
mod row;

pub use config::ClickHouseConfig;
pub use repository::ClickHouseEventRepository;
