# Memoria de las instancias

Todo está acotado: la memoria del procesador no depende del volumen de entrada, sino de
su configuración.

| Límite | Controla |
|---|---|
| `NATS_FETCH_BATCH` | mensajes crudos en vuelo desde NATS |
| capacidad del canal de entrada | eventos entre ingesta y batcher |
| `PIPELINE_BATCH_ROWS`, `PIPELINE_BATCH_BYTES` | tamaño de cada lote |
| `PIPELINE_WRITERS` (K) | lotes retenidos a la vez hasta su ack |
| `NATS_MAX_ACK_PENDING` | techo global de toda la flota |

## Estimación por pod

```
mem ≈ filas_lote × (K + 1) × bytes_evento × amplificación + base
    ≈ 50k × 3 × 1 KB × 3 + 100 MB ≈ 550 MB  → limit 1 GiB
```

> [!NOTE]
> Amplificación ≈ 3: bytes crudos (retenidos para ack/DLQ) + `SourceEvent` + `EventRow`.
> El buffer RowBinary no cuenta entero: el cliente lo envía por trozos mientras escribe.

## Prácticas

| Práctica | Motivo | Estado |
|---|---|---|
| `requests.memory = limits.memory` (QoS *Guaranteed*) | el nodo no desaloja el pod por sorpresa | ⏳ manifiestos k8s |
| `jemalloc` / `mimalloc` | menos fragmentación en servicios de larga vida | ⏸ si el profiling lo pide |
| `EventRow<'a>` con préstamos y etiquetas `&'static str` | menos copias (hoy `label()` asigna) | ⏸ si el profiling lo pide |
| soltar filas tras el INSERT | hasta entonces basta evento + handle de ack | ✅ |
