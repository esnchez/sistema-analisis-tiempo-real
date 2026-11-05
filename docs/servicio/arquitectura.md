# Visión general

## Flujo de datos

```
clientes ──HTTP──▶ event-gateway ──▶ NATS JetStream ──▶ data-processor ──▶ ClickHouse
(web, apps)        POST /v1/events   stream EVENTS      intake → batcher     tabla ancha
                   no valida         │                  → writers ×K         `events`
                                     │                        │ inválido / rechazado
                                     └──── stream EVENTS_DLQ ◀┘
```

| Servicio | Binario | Responsabilidad | Detalle |
|---|---|---|---|
| gateway | `event-gateway` | aceptar eventos de clientes y publicarlos | [gateway](gateway.md) |
| NATS | — | desacoplar entrada y procesamiento; retener el backlog | [topología](topologia-nats.md) |
| procesador | `data-processor` | validar, agrupar en lotes y persistir; apartar lo inválido | [concurrencia](concurrencia.md) |
| ClickHouse | — | almacenamiento y consultas OLAP | [tabla](tabla-events.md) |

> [!NOTE]
> Gateway y procesador comparten crate e imagen, pero se despliegan por separado: escalan con
> cosas distintas ([despliegue](../operaciones/despliegue-y-escalado.md)). Dentro del procesador,
> las etapas son tareas de un mismo proceso, no microservicios.

## Capas (hexagonal)

```
infrastructure ──▶ application ──▶ domain
      (las dependencias solo apuntan hacia dentro)
```

| Capa | Contiene | No puede |
|---|---|---|
| `domain` | tipos validados al construirse/deserializarse | hacer I/O |
| `application` | comandos CQS, handlers, puertos (traits) | conocer HTTP, NATS o ClickHouse |
| `infrastructure` | adaptadores, pipeline, configuración | contener reglas de negocio |

| Comando | Handler | Puerto | Adaptador |
|---|---|---|---|
| `PublishEvents` | `PublishEventsHandler` (no valida) | `EventPublisher` | `NatsEventPublisher` |
| `RecordEvents` | `RecordEventsHandler` (dedup por `event_id`) | `EventRepository` | `ClickHouseEventRepository` |

## Estructura

```
sistema-analisis-tiempo-real/
├── docker-compose.yml          NATS · ClickHouse · processor · gateway (+ perfiles loadtest y k6)
├── docs/                       esta documentación
├── k8s/                        manifiestos del sistema (Kubernetes)
│   └── monitoring/             Grafana, Prometheus, alertas y valores de kube-prometheus-stack
├── loadtest/                   k6: script, Job de Kubernetes, NodePorts para k6 externo y monitorización de Compose
└── processor/                  crate Rust `data-processor`
    ├── Dockerfile              imagen con los dos binarios
    ├── config/clickhouse.sql   esquema de `events`
    ├── src/
    │   ├── main.rs             binario data-processor
    │   ├── bin/event-gateway.rs
    │   ├── domain/             SourceEvent, payloads, ids, valores, EventTimeWindow
    │   ├── application/
    │   │   ├── command/        PublishEvents · RecordEvents · CommandHandler
    │   │   └── port/           EventPublisher · EventRepository
    │   └── infrastructure/
    │       ├── http/           router del gateway, lectura tolerante de eventos
    │       ├── nats/           topología, publicador, flujo de mensajes, DLQ
    │       ├── pipeline/       intake, batcher, writers, retry, breaker
    │       ├── clickhouse/     EventRow, repositorio, config
    │       ├── telemetry.rs    logs y métricas OTLP
    │       ├── heartbeat.rs    latido para la liveness probe
    │       ├── env.rs          variables de entorno
    │       └── signal.rs       señales de parada
    └── tests/integration/      pruebas contra servicios reales (refleja src/)
```
