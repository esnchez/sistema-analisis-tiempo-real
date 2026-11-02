# Estado del proyecto

Qué está hecho y qué falta. El detalle del código está en [servicio/](servicio), el despliegue,
la monitorización y las pruebas con sus resultados en [operaciones/](operaciones)
([pruebas](operaciones/pruebas.md)).

## Objetivos del README

A fecha de 2026-11-03, con las pruebas del 2026-11-02.

| Objetivo | Estado |
|---|---|
| Fase 1: arquitectura y esquema de datos | ✅ [arquitectura](servicio/arquitectura.md), [tabla `events`](servicio/tabla-events.md) |
| Fase 2: ingesta y almacenamiento en ClickHouse | ✅ [gateway](servicio/gateway.md) HTTP → NATS, tabla ancha `ReplacingMergeTree` |
| Fase 3: procesador en Rust y flujos con NATS | ✅ JetStream con dedup en 4 capas, DLQ, bisección, circuit breaker ([garantías](servicio/garantias-de-entrega.md)) |
| Fase 4: desplegar en Kubernetes y medir | 🟡 desplegado y medido en minikube con Prometheus, Grafana y alertas; EKS sin hacer |
| 2 000 req/s sostenidos | ✅ en Compose (2 000 en meseta) · 🟡 en minikube 1 870: la CPU de 4 núcleos se satura, con k6 dentro ([pruebas](operaciones/pruebas.md)) |
| p99 < 200 ms | ✅ en Compose (p95 44,5 ms) · ❌ en minikube (p99 299 ms, p95 183 ms) por la CPU compartida |
| Tasa de error < 0,1 % | ✅ 0 errores en todas las pruebas; conciliación exacta, sin pérdidas ni duplicados |
| Pruebas unitarias y de integración (NATS → ClickHouse) | ✅ 63 unitarias, 5 de integración en Compose y en el clúster |
| Punto de saturación ~3 500 req/s | ⚪ sin medir: en local satura antes la CPU compartida; necesita k6 fuera del clúster |
| 2 TB/día (~23 MB/s, ~38 000 eventos/s de ~600 B) | ⚪ no demostrable en local: máximo medido ~6 400 eventos/s en Compose con el procesador a ~0,4 núcleos; requiere escalar en un clúster real |
| Respuesta analítica de 10 min a 500 ms | ⚪ sin medir: frescura de ~2 s (dato consultable), pero no hay lado de lectura (consultas) ni línea base |
| Temporal | ⚪ sin implementar: diseñado fuera del camino caliente (RGPD, reproceso de la DLQ, *backfills*; [fallos](servicio/fallos-y-reintentos.md#temporal-fuera-del-camino-caliente)) |
| Despliegue en EKS con Terraform | ⚪ sin hacer: el README tiene solo un esbozo (EKS 1.21) |
| Alta disponibilidad y escalado | 🟡 2 réplicas fijas de gateway y procesador, parada ordenada y probes; sin HPA/KEDA ni ClickHouse/NATS replicados |
| Consistencia de datos | ✅ sin transacciones: at-least-once + idempotencia (ver [decisiones](servicio/decisiones.md)) |

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
- [x] Fetch acotado: lotes *pull* de `NATS_FETCH_BATCH` y canal de entrada lleno ⇒ deja de consumir
- [x] *AckWait* ≫ cola + lote + reintentos (aviso al arrancar si no)
- [x] *MaxAckPending* configurable (≥ pods × lote × (K + 3))

**Operación**
- [x] `Taskfile.yml` como punto de entrada único (entornos, despliegue, pruebas y carga), también desde PowerShell
- [x] Parada ordenada (procesador y gateway)
- [x] Prueba de carga de extremo a extremo con conciliación (Docker Compose y Kubernetes)
- [x] Entorno de pruebas en Kubernetes con Prometheus, Grafana y alertas ([Kubernetes](operaciones/kubernetes.md))
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
| Vistas materializadas por minuto | ⏳ | [escala](servicio/clickhouse-a-escala.md) |
| Anonimización en `account_deleted` (RGPD) | ⏳ | workflow de Temporal |
| Autoescalado con KEDA (lag de JetStream) | ⏳ | hoy réplicas fijas en `k8s/` |
| Trazas (OTLP) con `traceparent` en NATS | ⏳ | hoy solo métricas |
