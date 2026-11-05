# Garantías de entrega y deduplicación

Cómo se consigue que cada evento cuente **exactamente una vez** en el análisis sin
transacciones entre NATS y ClickHouse. Términos en [glosario](../glosario.md).

## Requisitos que guían el diseño

| Requisito | Valor | Consecuencia |
|---|---|---|
| Volumen | ~2 TB/día ≈ 23 MB/s medio | ~44k eventos/s con los 524 B medidos por evento · picos ×3 ≈ 130k/s ([extrapolación](../README.md#extrapolación-a-los-objetivos-del-readme)) |
| Frescura | evento consultable en segundos | lotes cortos en tiempo (1–2 s) |
| Pérdida de datos | ninguna | *ack* solo tras persistir |
| Duplicados | invisibles para el análisis | escritura *idempotente* por `event_id` |
| Orden | **no requerido** | paralelismo libre, sin particionar por clave |

> [!NOTE]
> Las cifras son estimaciones de partida; se validan con las [pruebas de carga](../operaciones/pruebas.md).

> [!IMPORTANT]
> **Idea central:** el análisis agrega por `occurred_at` (*tiempo de evento*), no por orden de
> llegada. Sin orden estricto, cualquier pod —y cualquier tarea dentro de un pod— puede
> procesar cualquier mensaje. Escalar = añadir réplicas o tareas.

## At-least-once + sumidero idempotente

```
exactly-once real (end-to-end)  ✗  NATS y ClickHouse no comparten transacción
at-least-once + idempotencia    ✓  = "efectivamente una vez"
```

Un duplicado puede aparecer en 3 puntos; cada uno tiene su defensa:

```
 Cliente ──(1)──▶ gateway ──▶ JetStream ──(2)──▶ procesador ──(3)──▶ ClickHouse
 reintenta la                  reentrega tras      reintenta el
 petición                      AckWait / caída     INSERT
```

| # | Origen | Defensa | Dónde | Ventana |
|---|---|---|---|---|
| 1 | el cliente (o el gateway) reintenta | `Nats-Msg-Id = event_id` → JetStream descarta | gateway | `duplicate_window` (2 min) |
| 2 | ack perdido, pod caído, *AckWait* vencido | dedup en el lote (`HashSet<EventId>`) + capas 3–4 | `RecordEventsHandler` | lote |
| 3 | `INSERT` reintentado tras timeout ambiguo | `insert_deduplication_token` = XOR + suma de los `event_id` + tamaño | repositorio ClickHouse | `*_deduplication_window` |
| 4 | todo lo anterior falla | `ReplacingMergeTree` colapsa por clave de ordenación al fusionar | tabla `events` | eventual, sin límite |

La DLQ también es idempotente: `Nats-Msg-Id = <stream>:<secuencia>` del mensaje original, con una
`duplicate_window` de 10 min > *AckWait* ([fallos](fallos-y-reintentos.md#dlq)).

> [!IMPORTANT]
> **Reglas que no se pueden romper**
> - **Un lote es inmutable una vez formado** (`Arc<[SourceEvent]>`). Un reintento envía las
>   mismas filas con el mismo token; si no, la capa 3 no actúa.
> - **La clave de ordenación sale 100 % del evento**, nunca de la hora de ingesta. Si no, dos
>   copias tendrían claves distintas y nunca se colapsarían.

> [!WARNING]
> En tablas **no replicadas** la dedup de inserción viene desactivada
> (`non_replicated_deduplication_window = 0`). El esquema la activa (`= 1000`); las tablas
> `Replicated*` usan `replicated_deduplication_window`, activa por defecto.

> [!TIP]
> Consultas exactas → `FINAL` o `uniqExact(event_id)`. Con partición determinista,
> `do_not_merge_across_partitions_select_final` abarata `FINAL`.

## Relacionado

- [Topología de NATS](topologia-nats.md) · [Fallos y reintentos](fallos-y-reintentos.md) · [Tabla de ClickHouse](tabla-events.md)
