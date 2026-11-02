# Entorno local (Docker Compose)

Para desarrollo rápido. El entorno de pruebas con monitorización completa es
[Kubernetes](kubernetes.md).

## Servicios

| Servicio | Imagen | Puertos | Perfil |
|---|---|---|---|
| `nats` | `nats:2.14-alpine` (JetStream) | 4222 clientes · 8222 monitorización | — |
| `clickhouse` | `clickhouse/clickhouse-server:26.8` (LTS) | 8123 HTTP · 9000 nativo | — |
| `processor` | `rtas-processor:dev` (`data-processor`) | — | — |
| `gateway` | `rtas-processor:dev` (`event-gateway`) | 8080 | — |
| `prometheus` | `prom/prometheus:v3.15.0` | 9090 | `loadtest` |
| `grafana` | `grafana/grafana:13.2.3` | 3000 (sin login) | `loadtest` |
| `k6` | `grafana/k6:2.3.0` | — | `loadtest` (se ejecuta con `run`) |

## Comandos

```bash
docker compose up -d --build                         # pipeline completo
docker compose --profile loadtest up -d --build      # + Prometheus y Grafana (config en k8s/monitoring/)
docker compose down -v && docker compose up -d       # recrear esquema y volúmenes
docker compose logs -f processor gateway             # logs

# enviar un evento a mano
curl -X POST localhost:8080/v1/events -H 'content-type: application/json' -d '[{"event_id":"0199b8a2-6c1e-7a51-9f3d-2b8c4e5f6a70","user_id":"u-1","occurred_at":"2026-10-07T10:00:00Z","source":"user_data","event":{"type":"account_deleted"}}]'

# mirar cada parte
curl 'localhost:8222/jsz?streams=true&consumers=true'                                    # NATS: lag, acks, DLQ
docker compose exec clickhouse clickhouse-client --password clickhouse -q 'SELECT event_type, count() FROM events GROUP BY 1'
```

## Pruebas

```bash
cd processor
cargo test                                                           # unitarias (sin servicios)
CLICKHOUSE_PASSWORD=clickhouse cargo test --test integration -- --ignored   # requiere nats + clickhouse
```

## Variables de entorno

| Prefijo | Servicio | Documentación |
|---|---|---|
| `NATS_*` | gateway, procesador | [topología](../processing/nats-topology.md) |
| `CLICKHOUSE_*` | procesador | `infrastructure/clickhouse/config.rs` |
| `PIPELINE_*` | procesador | [concurrencia](../processing/concurrency.md) |
| `GATEWAY_*` | gateway | [gateway](../ingestion/gateway.md) |
| `RUST_LOG` | ambos | filtro de `tracing` (`info`, `debug`…) |
| `OTEL_*` | ambos | exportación de métricas por OTLP; sin `OTEL_EXPORTER_OTLP_ENDPOINT` no se exportan ([observabilidad](observability.md)) |
| `HEARTBEAT_FILE` | procesador | archivo del latido para la liveness probe ([Kubernetes](kubernetes.md)) |
