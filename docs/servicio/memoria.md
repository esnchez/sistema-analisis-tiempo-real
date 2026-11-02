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
mem ≈ filas_lote × (K + 3) × bytes_evento × amplificación + canal + prefetch + base
    ≈ 50k × 5 × 1 KB × 3 + 100 MB ≈ 850 MB   (peor caso, eventos de 1 KB)
```

Lotes completos que pueden existir a la vez: K dentro de las tareas de escritura, 1 recibido
por el bucle de writers esperando un permiso del `Semaphore`, 1 en el canal `mpsc(1)` y 1 en el
batcher bloqueado en `send`. A eso se suman hasta 10 000 eventos en el canal de entrada y
`NATS_FETCH_BATCH` (1 000) mensajes del *prefetch*.

> [!WARNING]
> El límite del pod en Kubernetes es **768 Mi** (`k8s/processor.yaml`), por debajo del peor
> caso teórico con eventos de 1 KB. Los eventos reales de la prueba de carga son mucho más
> pequeños: en la prueba de carga en Kubernetes el procesador no pasó de **13 MiB**
> ([pruebas](../operaciones/pruebas.md)), porque los lotes se cortan por antigüedad. Si los eventos
> crecen, bajar `PIPELINE_BATCH_ROWS` o subir el límite.

> [!NOTE]
> Amplificación ≈ 3: bytes crudos (retenidos para ack/DLQ) + `SourceEvent` + `EventRow`.
> El buffer RowBinary no cuenta entero: el cliente lo envía por trozos mientras escribe.

## Prácticas

| Práctica | Motivo | Estado |
|---|---|---|
| `requests.memory = limits.memory` (QoS *Guaranteed*) | el nodo no desaloja el pod por sorpresa | ✅ 768 Mi en `k8s/processor.yaml` |
| `jemalloc` / `mimalloc` | menos fragmentación en servicios de larga vida | ⏸ si el profiling lo pide |
| `EventRow<'a>` con préstamos y etiquetas `&'static str` | menos copias (hoy `label()` asigna) | ⏸ si el profiling lo pide |
| soltar filas tras el INSERT | hasta entonces basta evento + handle de ack | ✅ |
