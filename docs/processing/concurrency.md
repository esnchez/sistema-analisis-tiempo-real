# Concurrencia dentro de un pod

Cómo un único proceso consume más mensajes de forma segura. Código: `infrastructure/pipeline/`.

## Dónde ayuda (y dónde no)

| Etapa | Cuello de botella | ¿Concurrencia? |
|---|---|---|
| fetch (NATS) | red | no: 1 tarea con *prefetch* basta |
| decode JSON + validación | CPU | poco: serde_json procesa cientos de MB/s por núcleo; la flota necesita ~23 MB/s. **Medir antes** |
| INSERT en ClickHouse | espera de red + servidor (50–500 ms/lote) | **sí: aquí está la ganancia** |
| ack | red | va dentro de cada tarea de inserción |

> [!IMPORTANT]
> La ganancia viene del **solapamiento** (*pipelining*): mientras el lote 1 se inserta, el lote 2
> ya se está llenando. **No** de muchas inserciones a la vez: ClickHouse quiere pocas y grandes,
> así que el número de writers `K` se queda pequeño (2–4).

## Pipeline

```
 ┌──────────────────┐  mpsc   ┌─────────────┐  mpsc(1)  ┌────────────────┐   ┌───────┐
 │ intake           │────────▶│   batcher   │──────────▶│ writers ×K     │──▶│  ack  │
 │ fetch+decode+    │ acotado │ filas/bytes │           │ Semaphore(K)   │   │       │
 │ valida (N = 1)   │         │  /antigüedad│           │ INSERT+retry   │   └───────┘
 └──▲──────┬────────┘         └─────────────┘           └───────┬────────┘
    │      │ inválido / fuera de ventana                        │ rechazo de datos
    │      ▼                                                    ▼
    │   DLQ + term                                       bisección → DLQ
    │
    └── backpressure: canales llenos ⇒ deja de pedir · circuit breaker abierto ⇒ deja de pedir
        CancellationToken ⇒ parada ordenada de todas las etapas
```

| Etapa | Archivo | Paralelismo | Corte / límite |
|---|---|---|---|
| intake | `intake.rs` | 1 tarea | pide solo el hueco libre (`NATS_FETCH_BATCH`) |
| batcher | `batcher.rs` | 1 tarea | **filas ≥ 50k · bytes ≥ 32 MB · antigüedad ≥ 2 s** (lo primero) |
| writers | `writer.rs` | K tareas (`Semaphore`) | cada una = 1 comando `RecordEvents` |
| ack | `writer.rs` | dentro del writer | tras `INSERT` OK |

> [!NOTE]
> Decode y validación van en la tarea de ingesta (N = 1): no compensa paralelizarlos sin un
> profiling que lo justifique.

## Salvaguardas

| Riesgo al añadir concurrencia | Salvaguarda |
|---|---|
| Memoria sin límite | canales acotados + `Semaphore(K)` → techo ≈ canales + lote × (K + 1) |
| **Acks fuera de orden**: el lote 2 termina antes que el 1 | `AckPolicy::Explicit`. **Nunca `All`**: confirmaría mensajes del lote 1 aún no guardados |
| Ack antes de persistir | los handles de ack viajan **dentro** de la tarea del writer; solo se usan tras `Ok` |
| La cola local consume *AckWait* → reentrega en curso | no sobre-pedir; *AckWait* ≫ `PipelineConfig::max_hold_time()` (aviso al arrancar si no) |
| Estado mutable compartido | ninguno: datos por canales; handler tras `Arc`; `Client` de ClickHouse con pool propio |
| Reintentos ocupan un writer | deseado: K reintentando ⇒ backpressure natural; *circuit breaker* para fallar rápido |
| El reintento necesita el lote | `RecordEvents { events: Arc<[SourceEvent]> }`: clon barato y lote idéntico |
| `panic` en una tarea | `JoinSet` lo reporta; sin ack ⇒ reentrega ⇒ dedup |
| SIGTERM pierde un lote | parar fetch → cerrar lote → esperar `JoinSet` → ack → salir |
| Más pods ⇒ demasiados INSERT/s | escalar **tamaño de lote**, no K |

> [!WARNING]
> El tiempo que un mensaje pasa en un canal local **cuenta para *AckWait***. Un *prefetch*
> generoso con writers lentos produce reentregas de mensajes que todavía se están procesando.

## Parámetros de ajuste (en este orden)

| # | Parámetro | Variable | Equilibrio |
|---|---|---|---|
| 1 | tamaño / antigüedad del lote | `PIPELINE_BATCH_ROWS` · `_BYTES` · `_AGE_MS` | rendimiento ↔ frescura |
| 2 | K writers | `PIPELINE_WRITERS` (2) | subir solo si domina la latencia de INSERT |
| 3 | capacidad del canal de entrada | `PipelineConfig::channel_capacity` | lo justo para que no esperen los writers |
| 4 | *MaxAckPending* | `NATS_MAX_ACK_PENDING` | ≥ pods × filas_lote × (K + 1) |

Entre pods: todos comparten el **mismo consumidor durable**. Escalar fuera = añadir réplicas
([despliegue](../operations/deployment-and-scaling.md)).
