# Estado de implementación

## Checklist — no podemos olvidar

**Entrega y dedup**
- [x] Ack **solo** tras INSERT confirmado
- [x] `AckPolicy::Explicit` (nunca `All` con lotes concurrentes)
- [x] Lote inmutable (`Arc<[SourceEvent]>`) + `insert_deduplication_token` en reintentos
- [x] Dedup por `event_id` dentro del lote
- [x] Clave de ordenación 100 % derivada del evento
- [x] `Nats-Msg-Id = event_id` al publicar (gateway)

**Fallos**
- [x] Mensajes inválidos → DLQ + `term`, nunca bloquean el lote
- [x] DLQ idempotente (`Nats-Msg-Id` del mensaje original; ventana > *AckWait*)
- [x] Bisección de lotes rechazados
- [x] Ventana de `occurred_at` aceptada
- [x] Circuit breaker; nada de `nak` masivo

**Concurrencia y memoria**
- [x] Canales acotados de extremo a extremo + `Semaphore(K)`
- [x] Fetch solo del hueco libre
- [x] *AckWait* ≫ cola + lote + reintentos (aviso al arrancar si no)
- [x] *MaxAckPending* configurable (≥ pods × lote × (K + 1))

**Operación**
- [x] Parada ordenada (procesador y gateway)
- [x] Prueba de carga de extremo a extremo con conciliación (Docker Compose y Kubernetes)
- [x] Entorno de pruebas en Kubernetes con Prometheus, Grafana y alertas ([Kubernetes](operations/kubernetes.md))
- [x] Liveness del procesador independiente de dependencias (latido + probe `exec`)
- [ ] Sharding determinista si se pasa a clúster
- [ ] Backlog de JetStream dimensionado para la peor caída
- [x] Métricas propias de gateway y procesador (OTLP) + alertas (`PrometheusRule`)

## Pendiente

| Tema | Estado | Nota |
|---|---|---|
| `+WIP` para lotes largos | ⏳ | hoy *AckWait* (120 s) ≫ hold time (~10 s) |
| `EventRow` con préstamos, allocator alternativo | ⏸ | solo si el profiling lo justifica |
| Lado de lectura CQS (queries + handlers) | ⏳ | |
| Vistas materializadas por minuto | ⏳ | [escala](storage/clickhouse-at-scale.md) |
| Anonimización en `account_deleted` (RGPD) | ⏳ | workflow de Temporal |
| Autoescalado con KEDA (lag de JetStream) | ⏳ | hoy réplicas fijas en `k8s/` |
| Trazas (OTLP) con `traceparent` en NATS | ⏳ | hoy solo métricas |
