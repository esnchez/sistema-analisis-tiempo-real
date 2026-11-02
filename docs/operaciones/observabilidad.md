# Observabilidad

## Cómo llegan las métricas

```
gateway, procesador ──OTLP/gRPC──▶ OpenTelemetry Collector ──:8889──▶ Prometheus ──▶ Grafana
NATS (exporter :7777) · ClickHouse (:9363) · pods (cAdvisor) ───────▲
```

Los servicios **empujan** sus métricas (OTLP/gRPC); no abren puertos para ellas. Se activan con
`OTEL_EXPORTER_OTLP_ENDPOINT` u `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`; sin ninguna de las dos (p. ej.
en Docker Compose) no se exportan.
Despliegue y dashboards: [Kubernetes](kubernetes.md). Código: `infrastructure/telemetry.rs`.

| Variable | Defecto | Efecto |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | — | colector (`http://otel-collector:4317`); sin ella ni `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`, métricas desactivadas |
| `OTEL_METRIC_EXPORT_INTERVAL` | `60000` | ms entre envíos (10 s en `k8s/`) |
| `OTEL_RESOURCE_ATTRIBUTES` | — | `service.instance.id=<pod>` → etiqueta `instance`; en `k8s/` también `k8s.pod.name` |

## Métricas propias

En Prometheus: puntos → `_`, más el sufijo de unidad o `_total`. `job` = servicio,
`instance` = pod.

| Métrica (Prometheus) | Tipo | Etiquetas | Qué mide |
|---|---|---|---|
| `processor_events_received_total` | contador | | eventos válidos leídos de NATS |
| `processor_events_stored_total` | contador | | eventos persistidos y confirmados |
| `processor_dead_letters_total` | contador | `reason` | mensajes apartados en la DLQ |
| `processor_batches_abandoned_total` | contador | | lotes que agotaron los reintentos (se reentregarán) |
| `processor_batch_rows` | histograma | | filas por lote |
| `processor_batch_size_bytes` | histograma | | bytes por lote |
| `processor_insert_duration_seconds` | histograma | `outcome` (`ok`, `transient`, `rejected`) | cada intento de `INSERT` |
| `processor_event_freshness_seconds` | histograma | | `occurred_at` → persistido |
| `processor_writers_busy` | gauge | | lotes escribiéndose (≤ K) |
| `processor_breaker_open` | gauge | | 1 = circuit breaker abierto |
| `processor_nats_max_ack_pending` | gauge | | *MaxAckPending* configurado (`NATS_MAX_ACK_PENDING`); el exporter de NATS no lo publica |
| `http_server_request_duration_seconds` | histograma | `http_route`, `http_request_method`, `http_response_status_code` | peticiones al gateway |
| `gateway_events_accepted_total` | contador | | eventos confirmados por JetStream |

> [!NOTE]
> Además: estado de JetStream (`jetstream_consumer_num_pending`, `…_num_ack_pending`,
> `…_num_redelivered`, `jetstream_stream_total_bytes`) desde el exporter de NATS, y métricas de
> ClickHouse (`ClickHouseProfileEvents_InsertedRows`, `ClickHouseAsyncMetrics_MaxPartCountForPartition`).

## Alertas

Definidas en `k8s/monitoring/alerts.yaml` (`PrometheusRule`).

| Alerta | Condición | Durante | Severidad |
|---|---|---|---|
| `RtasConsumerLagGrowing` | lag > 10 000 y creciendo | 10 min | warning |
| `RtasAckPendingNearLimit` | sin ack > 80 % de `processor_nats_max_ack_pending` | 5 min | warning |
| `RtasCircuitBreakerOpen` | algún procesador con el circuito abierto | 1 min | critical |
| `RtasDeadLetterRateHigh` | DLQ > 0,1 % de los eventos | 10 min | warning |
| `RtasFreshnessHigh` | frescura p99 > 10 s | 10 min | warning |
| `RtasInsertLatencyHigh` | `INSERT` p99 > 5 s | 10 min | warning |
| `RtasProcessorMetricsMissing` | ningún procesador envía métricas (el colector conserva las series 5 min, así que salta a los ~10 min) | 5 min | critical |
| `RtasGatewayErrorRateHigh` | 5xx > 0,1 % | 5 min | critical |
| `RtasGatewayLatencyHigh` | p99 > 200 ms (objetivo del README) | 10 min | warning |
| `RtasClickHouseTooManyParts` | > 300 partes en una partición | 5 min | warning |
| `RtasJetStreamStorageHigh` | streams > 70 % de `max_file_store` | 5 min | warning |
| `RtasPodMemoryNearLimit` | memoria de un contenedor > 80 % de su límite | 5 min | warning |

> [!TIP]
> `event_id` como identificador de correlación en los logs. Para trazas de extremo a extremo,
> propagar `traceparent` como cabecera de NATS (OpenTelemetry).
