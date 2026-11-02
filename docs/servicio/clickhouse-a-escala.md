# ClickHouse a escala

Riesgos al crecer y cómo se cubren. El diseño de la tabla está en [tabla-events.md](tabla-events.md).

| Punto | Riesgo | Medida |
|---|---|---|
| Partes por inserción | un lote con eventos de muchos días crea **una parte por partición**; `max_partitions_per_insert_block` = 100 | ventana de `occurred_at` aceptada (−7 d / +5 min); el resto a DLQ |
| Demasiadas partes | inserciones pequeñas/frecuentes → `TOO_MANY_PARTS` | lotes grandes, pocos INSERT/s, vigilar `system.parts` |
| Sharding (clúster) | la dedup solo funciona **dentro de un shard** | clave de sharding determinista: `sipHash64(user_id)` |
| Réplicas | pérdida de un nodo | `ReplicatedReplacingMergeTree` + Keeper |
| Escritura en clúster | cola interna de la tabla `Distributed` | insertar en las tablas locales de cada shard, o `insert_distributed_sync=1` |
| Consultas en vivo | escanear la tabla ancha para dashboards | vistas materializadas → `AggregatingMergeTree` por minuto/contenido |
| Retención | crecimiento ilimitado | `TTL occurred_at + INTERVAL N DAY` o `DROP PARTITION` programado |

> [!CAUTION]
> **Nunca `rand()` como clave de sharding**: las copias de un mismo evento irían a shards
> distintos y jamás se colapsarían.

> [!NOTE]
> **Ritmo de inserción de la flota** = `pods × K / intervalo_flush`. ClickHouse recomienda lotes
> de 10k–100k filas y del orden de **1 INSERT/s por tabla**. Alternativa: `async_insert=1` +
> `wait_for_async_insert=1` (el servidor agrupa; ack solo tras su confirmación).
