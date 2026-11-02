# Tabla `events` (one big table)

Una única tabla ancha y desnormalizada para análisis OLAP sin joins.
Esquema: `processor/config/clickhouse.sql` · fila en Rust: `infrastructure/clickhouse/row.rs` (`EventRow`).
Conexión (`infrastructure/clickhouse/config.rs`): `CLICKHOUSE_URL` (`http://localhost:8123`),
`CLICKHOUSE_DATABASE` (`default`), `CLICKHOUSE_USER` (`default`), `CLICKHOUSE_PASSWORD` (vacía).

```
SourceEvent (anidado)                         fila de `events` (plana)
├─ event_id, user_id, occurred_at     ──▶     event_id · event_type · user_id · occurred_at · ingested_at
├─ context { platform, country… }     ──▶     session_id · platform · device_type · country …
├─ content { id, section, author… }   ──▶     content_id · content_section · content_author_id …   (copiado del CMS)
└─ event { type, …campos propios }    ──▶     engaged_secs · playback_action · ad_id · subscription_plan …
                                               (lo que no aplica al tipo → valor por defecto)
```

## Decisiones

| Decisión | Por qué |
|---|---|
| Tabla ancha única | las consultas OLAP filtran y agregan sin joins |
| Desnormalización (contenido + contexto en cada fila) | agregar por sección, autor o plataforma directamente |
| Columnas que no aplican → `''` / `0` / `false` / `[]` | serialización dispersa: casi no ocupan |
| `Nullable` solo si 0/vacío es ambiguo | `content_published_at`, `duration_secs` (directos), `bitrate_kbps`, `price_amount_minor` |
| Enums → `LowCardinality(String)` | compacto y sin `ALTER` al añadir variantes |
| Etiquetas de enum = nombre serde | lo almacenado coincide con el JSON de las fuentes |
| `country` (IP) ≠ `profile_country` (declarado) | dos significados distintos |
| `ReplacingMergeTree(ingested_at)` | colapsa duplicados por clave al fusionar ([garantías](garantias-de-entrega.md)) |
| `ORDER BY (event_type, toStartOfHour(occurred_at), user_id, event_id)` | consultas típicas por tipo y rango temporal; termina en `event_id` para deduplicar por evento |
| `PARTITION BY toDate(occurred_at)` | ~2 TB/día: borrar/mover días completos |
| `non_replicated_deduplication_window = 1000` | descarta el reintento de un INSERT ya escrito |
| `bloom_filter` en `content_id`, `session_id` | búsquedas puntuales fuera de la clave |

> [!TIP]
> Recuentos exactos: `FINAL` o `uniqExact(event_id)`; la deduplicación de
> `ReplacingMergeTree` es eventual.

## Añadir una columna o un tipo de evento

1. `config/clickhouse.sql`: `ALTER TABLE events ADD COLUMN … DEFAULT …` (solo cambios aditivos).
2. `EventRow` en `row.rs` + su mapeo en `from_event`.
3. Desplegar el procesador **después** del `ALTER` ([evolución del esquema](../operaciones/evolucion-del-esquema.md)).
