# Despliegue y escalado

| Servicio | Escala con | Hoy (`k8s/`) | Previsto |
|---|---|---|---|
| `event-gateway` | peticiones HTTP de clientes | 2 réplicas fijas detrás del Service | *HPA* por CPU / peticiones |
| `data-processor` | lag del consumidor | 2 réplicas fijas sobre el mismo consumidor durable | *KEDA* con el scaler de NATS JetStream (`num_pending`), HPA por CPU como respaldo |

> [!NOTE]
> Todavía no hay HPA ni `ScaledObject` en los manifiestos ([estado](../estado.md)).

## Procesador

| Tema | Diseño |
|---|---|
| Escalado vertical | runtime tokio multihilo, K writers ([concurrencia](../servicio/concurrencia.md)) |
| Parada ordenada (SIGTERM) | 1) parar fetch · 2) cerrar lote · 3) cada writer inserta y hace ack · 4) esperar a todos · 5) salir. Lo que quedaba en el *prefetch* se reentrega tras *AckWait* |
| `terminationGracePeriodSeconds` | > antigüedad máx. de lote + timeout de INSERT + margen: 60 s (en compose `stop_grace_period: 60s`) |
| Liveness | probe `exec` sobre un latido en archivo (`HEARTBEAT_FILE`): proceso y runtime vivos, sin servidor HTTP |
| Readiness | ninguna: es un consumidor *pull*, ningún Service le envía tráfico. El progreso se vigila con el lag del consumidor |
| Métricas | empujadas por OTLP al colector; el procesador no abre puertos ([observabilidad](observabilidad.md)) |
| Rolling update | `maxUnavailable: 0`, `maxSurge: 1`; lo no confirmado se reentrega |

> [!WARNING]
> La *liveness* **no** debe depender de ClickHouse ni NATS: una caída de la dependencia
> reiniciaría todos los pods en cascada sin arreglar nada.

## Gateway

| Tema | Diseño |
|---|---|
| Parada ordenada | `preStop` de 5 s (el Service deja de enviarle tráfico) → deja de aceptar conexiones y termina las peticiones en curso; `terminationGracePeriodSeconds: 30` |
| Rolling update | `maxUnavailable: 0`, `maxSurge: 1` |
| Liveness / readiness | `GET /health` |

## Imagen

Manifiestos y puesta en marcha en minikube: [Kubernetes](kubernetes.md).

`processor/Dockerfile`: una imagen con los dos binarios (`data-processor`, `event-gateway`);
cada servicio elige el suyo con `command`. Multi-etapa sobre Debian trixie, usuario sin privilegios.

## Seguridad y almacenamiento en `k8s/`

| Tema | Valor |
|---|---|
| Usuario | `runAsUser: 65534` (gateway, procesador), `10001` (colector) |
| Sistema de archivos | `readOnlyRootFilesystem: true`, capacidades eliminadas; el procesador escribe el latido en un `emptyDir` de 1 Mi en `/tmp` |
| Token de API | `automountServiceAccountToken: false` |
| Volúmenes | PVC de 10 Gi para NATS (JetStream con `max_file_store` 8 GB) y para ClickHouse |
| Imágenes de terceros | `nats:2.14-alpine`, `natsio/prometheus-nats-exporter:0.20.3`, `clickhouse/clickhouse-server:26.8`, `otel/opentelemetry-collector:0.164.0` |
