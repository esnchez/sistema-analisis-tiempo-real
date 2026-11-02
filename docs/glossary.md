# Glosario

Términos usados en la documentación, agrupados por tema.

## Entrega y mensajería

| Término | Significado |
|---|---|
| **At-least-once** | Garantía de que cada mensaje se entrega **una o más** veces. Nunca se pierde, pero puede duplicarse. |
| **Exactly-once** | Cada mensaje produce efecto exactamente una vez. Requiere transacciones entre origen y destino; aquí se aproxima con at-least-once + idempotencia. |
| **Idempotencia** | Aplicar una operación varias veces deja el mismo resultado que aplicarla una. Permite reintentar sin miedo. |
| **Deduplicación (dedup)** | Detectar y descartar copias del mismo evento, aquí por `event_id`. |
| **Ack** | Confirmación al broker de que un mensaje está procesado; deja de reentregarse. |
| **Nak** | Rechazo temporal: pide al broker que reentregue (opcionalmente con retraso). |
| **Term** | Rechazo definitivo: el broker no lo reentregará nunca. Para mensajes venenosos. |
| **+WIP (in progress)** | Señal de "sigo trabajando": reinicia el temporizador de *AckWait* sin confirmar. |
| **Redelivery (reentrega)** | El broker vuelve a enviar un mensaje no confirmado a tiempo. |
| **Mensaje venenoso** | Mensaje que nunca podrá procesarse (JSON inválido, datos fuera de dominio). Reintentarlo es inútil. |
| **DLQ (dead-letter queue)** | Destino aparte para mensajes que no se pudieron procesar; se revisan o reprocesan después. |
| **Replay / backfill** | Volver a consumir mensajes antiguos del stream, p. ej. tras corregir un bug. |
| **Frescura** | Tiempo entre que ocurre un evento y que es consultable. |
| **Lag** | Mensajes pendientes de consumir; mide cuánto va por detrás el procesador. |
| **Tiempo de evento** | Cuándo ocurrió (`occurred_at`), frente al tiempo de procesamiento (cuándo llegó). |
| **Sesionización** | Agrupar eventos de un usuario en sesiones según pausas de inactividad. |

## NATS JetStream

| Término | Significado |
|---|---|
| **Stream** | Registro persistente de mensajes, en disco y replicado. |
| **Subject** | "Dirección" jerárquica de un mensaje (`events.user_behavior.content_viewed`); admite comodines (`events.>`). |
| **Retención `limits`** | Los mensajes se conservan hasta un límite de edad/tamaño, aunque se hayan consumido. |
| **Retención `workqueue`** | Cada mensaje se borra al confirmarse; impide el replay. |
| **Consumidor durable** | Consumidor con nombre cuyo progreso guarda el servidor; sobrevive a reinicios y lo comparten varios pods. |
| **Consumidor pull** | El cliente pide lotes cuando tiene capacidad (frente a *push*, donde el servidor empuja). |
| **AckWait** | Tiempo máximo para confirmar un mensaje antes de que se reentregue. |
| **MaxAckPending** | Máximo de mensajes entregados y sin confirmar, para todo el consumidor. |
| **MaxDeliver** | Número máximo de entregas de un mensaje. |
| **BackOff** | Lista de retrasos entre reentregas sucesivas. Si se define, **sustituye a AckWait** en cada entrega. |
| **Duplicate window** | Ventana en la que JetStream descarta publicaciones con `Nats-Msg-Id` repetido. |
| **Advisory** | Evento de sistema que publica JetStream (p. ej. "mensaje agotó MaxDeliver"). |
| **Subject mapping** | Reescritura de subjects en el servidor; permite repartir en particiones deterministas. |

## ClickHouse

| Término | Significado |
|---|---|
| **OLAP** | Procesamiento analítico: consultas que agregan muchas filas y pocas columnas. |
| **Tabla ancha (one big table)** | Una única tabla con todas las columnas necesarias para analizar sin joins. |
| **Desnormalización** | Copiar datos relacionados (contenido, contexto) en cada fila en vez de referenciarlos. |
| **Parte (part)** | Fichero inmutable que crea cada INSERT; el servidor las fusiona en segundo plano. |
| **Merge (fusión)** | Proceso de fondo que combina partes; es cuando `ReplacingMergeTree` elimina duplicados. |
| **Partición** | División física de la tabla (aquí, por día); las partes nunca se fusionan entre particiones. |
| **Clave de ordenación** | `ORDER BY` de la tabla: orden físico, índice primario y clave de deduplicación. |
| **ReplacingMergeTree** | Motor que, al fusionar, conserva una sola fila por clave de ordenación (la de versión mayor). |
| **FINAL** | Modificador de consulta que aplica la deduplicación en el momento de leer. Más costoso. |
| **insert_deduplication_token** | Identificador de un INSERT; si se repite dentro de la ventana, el servidor lo ignora. |
| **LowCardinality** | Codificación por diccionario para columnas con pocos valores distintos. |
| **Serialización dispersa (sparse)** | Almacena solo los valores no por defecto de una columna mayoritariamente vacía. |
| **RowBinary** | Formato binario por filas con el que el cliente Rust envía los INSERT. |
| **Shard** | Porción horizontal de los datos en un clúster; cada shard guarda filas distintas. |
| **Réplica** | Copia completa de un shard para tolerar fallos. |
| **Keeper** | Servicio de coordinación de ClickHouse (equivalente a ZooKeeper) para replicación. |
| **Tabla Distributed** | Tabla lógica que reparte inserciones y consultas entre shards. |
| **Vista materializada** | Transformación que se ejecuta en cada INSERT y escribe en otra tabla (p. ej. agregados). |
| **AggregatingMergeTree** | Motor que guarda estados de agregación parciales y los combina al fusionar. |
| **TTL** | Regla de expiración: borra o mueve filas/particiones pasado un tiempo. |
| **async_insert** | Modo en que el servidor agrupa muchos INSERT pequeños antes de escribir partes. |

## Concurrencia (Rust / tokio)

| Término | Significado |
|---|---|
| **Pipelining** | Encadenar etapas que trabajan a la vez sobre elementos distintos (llenar el lote 2 mientras se inserta el 1). |
| **Tarea (task)** | Unidad ligera de trabajo asíncrono de tokio; miles por proceso sin coste de hilos. |
| **Canal `mpsc` acotado** | Cola entre tareas con capacidad fija; si está llena, el emisor espera. |
| **Backpressure** | Propagación de la lentitud aguas arriba: el productor frena porque el consumidor no da abasto. |
| **Semaphore** | Contador de permisos que limita cuántas tareas hacen algo a la vez (aquí, K inserciones). |
| **JoinSet** | Conjunto de tareas lanzadas que se pueden esperar juntas; reporta fallos y `panic`s. |
| **CancellationToken** | Señal compartida para pedir a todas las tareas que terminen ordenadamente. |
| **`select!`** | Espera a varias operaciones a la vez y actúa sobre la primera que termine. |
| **`spawn_blocking`** | Ejecuta trabajo CPU intensivo en un pool aparte para no bloquear el runtime. |
| **`Arc`** | Puntero con contador de referencias para compartir datos inmutables entre tareas sin copiarlos. |
| **`Send` / `Sync`** | Marcas de Rust: el tipo puede moverse / compartirse entre hilos con seguridad. |
| **Prefetch** | Pedir mensajes por adelantado para que la siguiente etapa no espere. |

## Resiliencia

| Término | Significado |
|---|---|
| **Fallo transitorio** | Fallo que puede desaparecer reintentando (red, sobrecarga). |
| **Fallo permanente** | Fallo que se repetirá siempre con los mismos datos (formato, esquema). |
| **Backoff exponencial** | Esperar cada vez el doble entre reintentos (1 s, 2 s, 4 s…). |
| **Jitter** | Aleatoriedad añadida al backoff para que muchos clientes no reintenten a la vez. |
| **Circuit breaker** | Interruptor que, tras varios fallos seguidos, deja de llamar a la dependencia un tiempo. |
| **Half-open** | Estado del circuit breaker en el que deja pasar una prueba para ver si la dependencia volvió. |
| **Bisección** | Partir un lote rechazado en mitades sucesivas hasta aislar las filas culpables. |
| **Parada ordenada (graceful shutdown)** | Terminar el trabajo en curso antes de salir, sin perder ni duplicar más de lo necesario. |

## Kubernetes y operación

| Término | Significado |
|---|---|
| **HPA** | Horizontal Pod Autoscaler: ajusta réplicas según CPU/memoria u otras métricas. |
| **KEDA** | Autoescalado por eventos; aquí, según el lag del consumidor de JetStream. |
| **QoS Guaranteed** | Clase de pod con `requests = limits`; es el último en ser desalojado. |
| **OOMKill** | El kernel mata el proceso por superar su límite de memoria. |
| **Liveness probe** | Comprobación de "el proceso está vivo"; si falla, Kubernetes reinicia el pod. |
| **Readiness probe** | Comprobación de "puede recibir trabajo"; si falla, se le deja de enviar tráfico. |
| **SIGTERM** | Señal que envía Kubernetes antes de parar un pod; inicia la parada ordenada. |
| **Fragmentación** | Memoria libre partida en huecos pequeños inutilizables; crece en procesos de larga vida. |
| **Profiling** | Medir dónde se gasta CPU/memoria antes de optimizar. |

## Ingesta y pruebas de carga

| Término | Significado |
|---|---|
| **Gateway de ingesta** | Servicio HTTP que recibe eventos de los clientes y los publica en NATS (`event-gateway`). |
| **202 Accepted** | Respuesta HTTP: la petición se aceptó para procesarse después (aquí, ya guardada en JetStream). |
| **Nats-Msg-Id** | Cabecera con la que JetStream reconoce y descarta publicaciones repetidas. |
| **VU (virtual user)** | Usuario simulado de k6; ejecuta iteraciones del script en paralelo con los demás. |
| **Arrival rate** | Ejecutor de k6 que fija las peticiones por segundo, independientemente de lo que tarden. |
| **Threshold** | Criterio de éxito de k6 (p. ej. `p(99)<200`); si no se cumple, la prueba falla. |
| **Remote write** | Protocolo con el que k6 empuja métricas a Prometheus en vez de que Prometheus las recoja. |
| **Conciliación** | Comprobar al final que todo lo aceptado está en ClickHouse o en la DLQ: ni pérdidas ni duplicados. |
