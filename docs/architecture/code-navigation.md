# Cómo navegar el código

Rutas relativas a `processor/src/`.

| Quiero… | Ir a |
|---|---|
| ver el formato JSON de un evento | `domain/event/mod.rs` (doc del módulo) |
| seguir un evento de punta a punta | `infrastructure/http/mod.rs` → `nats/publisher.rs` → `pipeline/intake.rs` → `batcher.rs` → `writer.rs` → `clickhouse/repository.rs` |
| añadir un tipo de evento | `domain/event/user_*.rs` → `infrastructure/clickhouse/row.rs` → `config/clickhouse.sql` |
| añadir una columna | `config/clickhouse.sql` + `EventRow` en `infrastructure/clickhouse/row.rs` |
| añadir un caso de uso de escritura | `application/command/<verbo_nombre>.rs` |
| añadir una dependencia externa | trait en `application/port/` + implementación en `infrastructure/` |
| cambiar qué acepta el gateway | `infrastructure/http/` (`GATEWAY_*`) |
| ajustar lotes, writers o reintentos | `infrastructure/pipeline/mod.rs` (`PIPELINE_*`) |
| cambiar streams, consumidor o DLQ | `infrastructure/nats/config.rs` (`NATS_*`) |
| decidir qué error de ClickHouse es transitorio | `infrastructure/clickhouse/repository.rs` (`DATA_ERROR_CODES`) |
| cambiar la conexión a ClickHouse | `infrastructure/clickhouse/config.rs` (`CLICKHOUSE_*`) |
| añadir o cambiar una métrica | `infrastructure/pipeline/metrics.rs` (procesador) · `infrastructure/http/metrics.rs` (gateway) |
| configurar logs y exportación OTLP | `infrastructure/telemetry.rs` (`OTEL_*`) |
| cambiar el despliegue | `k8s/` ([Kubernetes](../operations/kubernetes.md)) |

## Convenciones

| Qué | Regla |
|---|---|
| Comando / handler | verbo imperativo (`RecordEvents`) · `+Handler` |
| Puerto / adaptador | `<Entidad>Repository`, `<Entidad>Publisher` · `<Tecnología><Puerto>` |
| Tests unitarios | archivo hermano `foo_tests.rs` vía `#[path]` (en `mod.rs` → `tests.rs`) |
| Tests de integración | `tests/integration/` refleja `src/`, marcados `#[ignore]` |
| Idioma | código, comentarios y documentación en español |
