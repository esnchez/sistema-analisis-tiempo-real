# Estado del proyecto

Qué está hecho y qué falta. El detalle del código está en [servicio/](servicio), el despliegue,
la monitorización y las pruebas con sus resultados en [operaciones/](operaciones)
([pruebas](operaciones/pruebas.md)).

## Objetivos del README

A fecha de 2026-11-05, con las pruebas del 2026-11-02 y 2026-11-05. Réplicas necesarias por objetivo: [extrapolación](README.md#extrapolación-a-los-objetivos-del-readme).

| Objetivo | Estado |
|---|---|
| Fase 1: arquitectura y esquema de datos | ✅ [arquitectura](servicio/arquitectura.md), [tabla `events`](servicio/tabla-events.md) |
| Fase 2: ingesta y almacenamiento en ClickHouse | ✅ [gateway](servicio/gateway.md) HTTP → NATS, tabla ancha `ReplacingMergeTree` |
| Fase 3: procesador en Rust y flujos con NATS | ✅ JetStream con dedup en 4 capas, DLQ, bisección, circuit breaker ([garantías](servicio/garantias-de-entrega.md)) |
| Fase 4: desplegar en Kubernetes y medir | 🟡 desplegado y medido en minikube con Prometheus, Grafana y alertas; EKS sin hacer |
| 2 000 req/s sostenidos | ✅ en Compose · 🟡 en minikube, 1 938 req/s con k6 fuera del clúster. Bastan 2 pods de gateway y 2 de procesador ([resultados](README.md#lo-que-demuestran-las-pruebas), [extrapolación](README.md#extrapolación-a-los-objetivos-del-readme)) |
| p99 < 200 ms | ✅ en Compose el 2026-11-02 · ❌ en minikube (p99 553 ms): falta CPU en el portátil, no capacidad del pipeline; necesita CPU dedicada para confirmarse ([por qué](README.md#por-qué-falla-el-p99-en-local)) |
| Tasa de error < 0,1 % | ✅ 0 errores en todas las ejecuciones del escenario del README; solo la prueba de eventos con la CPU desbordada dio 0,8 % de `503`. Ninguna pérdida ni duplicado ([resultados](README.md#lo-que-demuestran-las-pruebas)) |
| Pruebas unitarias y de integración (NATS → ClickHouse) | ✅ 63 unitarias, 5 de integración en Compose y en el clúster |
| Punto de saturación ~3 500 req/s | 🟡 extrapolado, no medido: coherente con un nodo dedicado de 4 vCPU ([extrapolación](README.md#extrapolación-a-los-objetivos-del-readme)) |
| 2 TB/día (~44 000 eventos/s) | 🟡 extrapolado, no medido: ~16 núcleos y **cambios de configuración** antes de escalar (retención y disco de NATS, `NATS_MAX_ACK_PENDING`, disco de ClickHouse) ([extrapolación](README.md#extrapolación-a-los-objetivos-del-readme)) |
| Respuesta analítica de 10 min a 500 ms | 🟡 datos consultables a los ~2 s y agregaciones simples por debajo de 500 ms; las de cardinalidad necesitan vistas materializadas. Sin línea base de 10 min ([consultas](README.md#consultas-analíticas-objetivo-de-500-ms)) |
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
