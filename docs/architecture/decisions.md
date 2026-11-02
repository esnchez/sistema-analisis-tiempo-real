# Decisiones clave

Registro breve del *qué* y el *porqué*. El detalle está en el documento enlazado.

| # | Decisión | Por qué | Detalle |
|---|---|---|---|
| 1 | Arquitectura hexagonal | infraestructura intercambiable y testeable con fakes | [visión general](overview.md) |
| 2 | CQS: comando + handler, sin datos de vuelta | escritura separada de lectura | [visión general](overview.md) |
| 3 | Gateway HTTP delante de NATS | punto de entrada real de los clientes; k6 lo prueba con HTTP nativo | [gateway](../ingestion/gateway.md) |
| 4 | El gateway no valida contenido | los clientes envían datos erróneos u obsoletos; la validación vive en un solo sitio | [gateway](../ingestion/gateway.md) |
| 5 | At-least-once + dedup en 4 capas | "efectivamente una vez" sin transacciones entre NATS y ClickHouse | [garantías](../processing/delivery-guarantees.md) |
| 6 | Comando por **lote** | ClickHouse rinde con inserciones grandes | [concurrencia](../processing/concurrency.md) |
| 7 | Pipeline con canales acotados + K writers | solapa llenado e inserción; memoria limitada; backpressure | [concurrencia](../processing/concurrency.md) |
| 8 | Ack por mensaje y solo tras el INSERT | lotes concurrentes terminan en cualquier orden sin perder datos | [concurrencia](../processing/concurrency.md) |
| 9 | Errores: transitorio por defecto, rechazo solo por datos | un fallo del almacén espera en NATS; los datos malos van a la DLQ | [fallos](../processing/failure-handling.md) |
| 10 | Sin *BackOff* y *MaxDeliver* ilimitado | *BackOff* sustituye a *AckWait*; lo inválido ya va a la DLQ explícitamente | [topología](../processing/nats-topology.md) |
| 11 | Pipeline independiente del broker (`Delivery`) | se prueba entero con fakes en memoria | [concurrencia](../processing/concurrency.md) |
| 12 | Tabla ancha única + desnormalización | consultas OLAP sin joins | [tabla](../storage/clickhouse-table.md) |
| 13 | `ReplacingMergeTree`, clave terminada en `event_id` | duplicados colapsados por evento | [tabla](../storage/clickhouse-table.md) |
| 14 | Ventana de `occurred_at` (−7 d / +5 min) | eventos de muchos días crean una parte por partición | [escala](../storage/clickhouse-at-scale.md) |
| 15 | Validación en el dominio (newtypes) | si un `SourceEvent` existe, es válido | `domain/` |
| 16 | k6 → gateway en vez de k6 → NATS | la extensión `xk6-nats` está abandonada (2023) y no funciona con k6 actual | [pruebas de carga](../operations/load-testing.md) |
