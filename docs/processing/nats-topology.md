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
| `AckPolicy` | `explicit` | ack por mensaje; imprescindible con lotes concurrentes ([concurrencia](concurrency.md)) |
| *AckWait* | 120 s ≫ cola local + edad de lote + reintentos (~10 s) | evitar reentregas de lotes vivos |
| *MaxAckPending* | ≥ pods × filas_lote × (K + 1) · por defecto 500 000 | si es menor, los pods se quedan sin mensajes |
| *MaxDeliver* | `-1` (sin límite) | lo inválido va a la DLQ de forma explícita; lo transitorio se reintenta hasta que el almacén vuelva |
| *BackOff* | **no se usa** | ver aviso |
| DLQ | stream propio, fuera de `events.>` | el consumidor no relee lo apartado |

> [!CAUTION]
> Con *BackOff* definido, sus valores **sustituyen a *AckWait*** en cada entrega. Un
> `[1s, 5s, …]` reentregaría al segundo los mensajes de lotes aún en curso. El espaciado lo
> dan *AckWait* y el [circuit breaker](failure-handling.md).

> [!NOTE]
> **¿Particionar por `user_id`?** No mientras no haga falta orden. Si algún caso lo exige
> (p. ej. sesionización en streaming): *subject mapping* con particiones deterministas y un
> consumidor por partición. Preferible: sesionizar en ClickHouse.

## Quién crea qué

Todo es idempotente (`create_or_update`): cada réplica lo aplica al arrancar.

| Servicio | Crea |
|---|---|
| gateway | streams `EVENTS` y `EVENTS_DLQ` (`ensure_streams`) |
| procesador | streams + consumidor `processor` (`ensure_streams` + `ensure_consumer`) |
