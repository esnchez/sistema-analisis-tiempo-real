# Fallos y reintentos

Qué hace el procesador ante cada fallo. Código: `infrastructure/pipeline/{writer,retry,breaker}.rs`
y la clasificación en `infrastructure/clickhouse/repository.rs`.

## Clasificación

| Fallo | Ejemplos | Tipo | Acción |
|---|---|---|---|
| Mensaje venenoso | JSON inválido, `DomainError`, tipo de evento desconocido | permanente (mensaje) | DLQ + `term` · **no bloquea el lote** |
| Fuera de ventana | `occurred_at` < now − 7 d o > now + 5 min | permanente (mensaje) | DLQ + `term` ([por qué](../storage/clickhouse-at-scale.md)) |
| ClickHouse transitorio | red, timeout, 5xx, `TOO_MANY_PARTS` (252), `MEMORY_LIMIT_EXCEEDED` (241), tabla/columna inexistente, **cualquier código desconocido** | transitorio | reintento en proceso: mismo lote y token, backoff exponencial + jitter |
| ClickHouse rechaza los datos | `CANNOT_PARSE_TEXT` (6), `TYPE_MISMATCH` (53), `CANNOT_CONVERT_TYPE` (70), `INCORRECT_DATA` (117)… | permanente (lote) | **bisección** hasta aislar las filas malas → DLQ; se inserta el resto |
| Pod muere a mitad de lote | OOM, despliegue, nodo caído | — | nada: sin ack → reentrega tras *AckWait* → dedup |
| NATS desconecta | red | transitorio | reconexión del cliente; acks perdidos → reentrega → dedup |
| ClickHouse caído mucho tiempo | incidente | transitorio largo | circuit breaker abierto → sin fetch → backlog en JetStream |

> [!IMPORTANT]
> **Ante la duda, transitorio.** Un desajuste de esquema (despliegue a medias) no es culpa de
> los datos: clasificarlo como rechazo llevaría a la DLQ el lote entero vía bisección. Como
> transitorio, los mensajes esperan en JetStream y el circuit breaker avisa.

## Dónde reintentar

```
intento 1 ──✗──▶ backoff en proceso (0,5 s, 1 s, 2 s, 4 s; 5 intentos ≈ ≤ 8 s ≪ AckWait)
                       │ agotado
                       ▼
               circuit breaker ABIERTO (3 lotes seguidos) ── la ingesta deja de pedir 30 s
                       │ mensajes sin ack
                       ▼
               JetStream reentrega tras AckWait, sin límite de entregas
                       │
                       ▼
               el almacén vuelve → lote OK → ack · el reintento duplicado se descarta

DLQ solo para lo que nunca funcionará: venenosos, fuera de ventana, filas rechazadas
```

> [!TIP]
> **En proceso primero**: mantiene el lote idéntico (dedup de inserción efectiva) y es barato.
> Tras el enfriamiento el circuito queda *semiabierto*: el primer éxito lo cierra y el primer
> fallo lo reabre.

> [!CAUTION]
> **Nunca `nak` masivo.** Con ClickHouse caído, reentregar todo solo genera tormentas de
> tráfico. Abrir el circuito y dejar de pedir.

## DLQ

| Motivo (`Dlq-Reason`) | Origen | Subject |
|---|---|---|
| `invalid_event` | no deserializa como `SourceEvent` | `dlq.events.invalid_event` |
| `out_of_time_window` | `EventTimeWindow` | `dlq.events.out_of_time_window` |
| `rejected_by_store` | bisección hasta 1 fila | `dlq.events.rejected_by_store` |

Cabeceras: `Dlq-Reason`, `Dlq-Detail`, `Dlq-Original-Subject`; cuerpo = mensaje original.
El mensaje se termina (`term`) **solo después** de que la DLQ confirme; si la DLQ falla, se reentrega.

**Idempotente:** `Nats-Msg-Id` = posición del mensaje original (`<stream>:<secuencia>`), igual
en cada reentrega. Si el mensaje vuelve antes de terminarse (pod caído entre la DLQ y el `term`,
mensaje que esperó en el *prefetch* más que *AckWait*), JetStream descarta la segunda copia. La
`duplicate_window` de la DLQ (10 min) debe superar *AckWait* (120 s); si no, el procesador avisa al
arrancar.

## Capacidad del backlog

`23 MB/s × duración de la caída` → 1 h ≈ 83 GB · 24 h ≈ 2 TB.

> [!WARNING]
> Con `retention=limits`, JetStream **descarta lo más antiguo** al llenarse. Dimensionar
> `max_bytes`/`max_age` y el disco de NATS para la peor caída asumible, y alertar antes.

## Temporal: fuera del camino caliente

> [!IMPORTANT]
> Un workflow por evento hundiría el rendimiento. Temporal es para procesos largos y con estado:
> supresión RGPD (`account_deleted`), reproceso de la DLQ y *backfills*, migraciones coordinadas.
