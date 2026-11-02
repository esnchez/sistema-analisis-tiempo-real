# Topología de NATS JetStream

Streams, subjects y el consumidor compartido por las réplicas del procesador.
Código: `infrastructure/nats/topology.rs` · configuración: `infrastructure/nats/config.rs` (`NATS_*`).

```
 gateway ──publish──▶ stream EVENTS   (events.>)   limits · file · 72 h · dup window 2 min
                              │
                  consumer durable "processor"  (pull · ack explícito)
              ┌───────────────┼───────────────┐
            pod 1           pod 2           pod N        ← mismo consumidor: reparto automático
              │ inválidos / rechazados
              ▼
        stream EVENTS_DLQ (dlq.events.<motivo>)  limits · 30 días
```

| Elemento | Elección | Por qué |
|---|---|---|
| Subjects | `events.<source>.<type>` (o `events.unknown`) | filtrar/reenrutar por tipo sin decodificar |
| Retención | `limits` + `max_age` (no `workqueue`) | permite *replay* tras un bug y otros consumidores |
| Consumidor | *pull*, *durable*, compartido por todos los pods | cada pod pide lo que puede procesar |
| `AckPolicy` | `explicit` | ack por mensaje; imprescindible con lotes concurrentes ([concurrencia](concurrencia.md)) |
| *AckWait* | 120 s ≫ cola local + edad de lote + reintentos (~10 s) | evitar reentregas de lotes vivos |
| *MaxAckPending* | ≥ pods × filas_lote × (K + 3) · por defecto 500 000 | si es menor, los pods se quedan sin mensajes |
| *MaxDeliver* | `-1` (sin límite) | lo inválido va a la DLQ de forma explícita; lo transitorio se reintenta hasta que el almacén vuelva |
| *BackOff* | **no se usa** | ver aviso |
| DLQ | stream propio, fuera de `events.>` | el consumidor no relee lo apartado |

> [!CAUTION]
> Con *BackOff* definido, sus valores **sustituyen a *AckWait*** en cada entrega. Un
> `[1s, 5s, …]` reentregaría al segundo los mensajes de lotes aún en curso. El espaciado lo
> dan *AckWait* y el [circuit breaker](fallos-y-reintentos.md).

> [!NOTE]
> **¿Particionar por `user_id`?** No mientras no haga falta orden. Si algún caso lo exige
> (p. ej. sesionización en streaming): *subject mapping* con particiones deterministas y un
> consumidor por partición. Preferible: sesionizar en ClickHouse.

## Configuración

| Variable | Defecto | Qué controla |
|---|---|---|
| `NATS_URL` | `nats://localhost:4222` | servidor |
| `NATS_STREAM` | `EVENTS` | stream de eventos |
| `NATS_CONSUMER` | `processor` | consumidor durable |
| `NATS_REPLICAS` | `1` | réplicas de los streams |
| `NATS_ACK_WAIT_MS` | `120000` | *AckWait* |
| `NATS_MAX_ACK_PENDING` | `500000` | *MaxAckPending*; el procesador lo publica como `processor_nats_max_ack_pending` para la alerta `RtasAckPendingNearLimit` |
| `NATS_FETCH_BATCH` | `1000` | mensajes por petición *pull* |

Fijos en código (`NatsConfig::default`): subjects `events.>`, prefijo de publicación `events`,
`max_age` 72 h, *duplicate window* 2 min; DLQ `EVENTS_DLQ` con prefijo `dlq.events`, 30 días y
*duplicate window* de 10 min.

## Quién crea qué

Todo es idempotente (`create_or_update`): cada réplica lo aplica al arrancar.

| Servicio | Crea |
|---|---|
| gateway | streams `EVENTS` y `EVENTS_DLQ` (`ensure_streams`) |
| procesador | streams + consumidor `processor` (`ensure_streams` + `ensure_consumer`) |
