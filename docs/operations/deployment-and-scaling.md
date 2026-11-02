# Despliegue y escalado

| Servicio | Escala con | Cómo |
|---|---|---|
| `event-gateway` | peticiones HTTP de clientes | réplicas detrás de un balanceador; *HPA* por CPU / peticiones |
| `data-processor` | lag del consumidor | réplicas sobre el mismo consumidor durable; *KEDA* con el scaler de NATS JetStream (`num_pending`), HPA por CPU como respaldo |

## Procesador

| Tema | Diseño |
|---|---|
| Escalado vertical | runtime tokio multihilo, K writers ([concurrencia](../processing/concurrency.md)) |
| Parada ordenada (SIGTERM) | 1) parar fetch · 2) cerrar lote · 3) esperar INSERTs · 4) ack · 5) salir |
| `terminationGracePeriodSeconds` | > antigüedad máx. de lote + timeout de INSERT + margen (p. ej. 60 s; en compose `stop_grace_period: 60s`) |
| Liveness | probe `exec` sobre un latido en archivo (`HEARTBEAT_FILE`): proceso y runtime vivos, sin servidor HTTP |
| Readiness | ninguna: es un consumidor *pull*, ningún Service le envía tráfico. El progreso se vigila con el lag del consumidor |
| Métricas | empujadas por OTLP al colector; el procesador no abre puertos ([observabilidad](observability.md)) |
| Rolling update | `maxUnavailable: 0`, `maxSurge: 1`; lo no confirmado se reentrega |

> [!WARNING]
> La *liveness* **no** debe depender de ClickHouse ni NATS: una caída de la dependencia
> reiniciaría todos los pods en cascada sin arreglar nada.

## Gateway

| Tema | Diseño |
|---|---|
| Parada ordenada | deja de aceptar conexiones y termina las peticiones en curso |
| Liveness / readiness | `GET /health` |

## Imagen

Manifiestos y puesta en marcha en minikube: [Kubernetes](kubernetes.md).

`processor/Dockerfile`: una imagen con los dos binarios (`data-processor`, `event-gateway`);
cada servicio elige el suyo con `command`. Multi-etapa sobre Debian trixie, usuario sin privilegios.
