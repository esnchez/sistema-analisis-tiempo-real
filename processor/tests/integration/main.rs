//! Pruebas de integración contra servicios reales. Replica la estructura de `src/`.
//!
//! Requieren los contenedores de `docker-compose.yml` levantados, por eso se marcan con
//! `#[ignore]`:
//!
//! ```bash
//! docker compose up -d
//! CLICKHOUSE_PASSWORD=clickhouse cargo test --test integration -- --ignored
//! ```

mod infrastructure;
