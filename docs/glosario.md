# Glosario

Términos usados en la documentación, agrupados por tema. La última columna enlaza los documentos
donde aparece cada término.

## Entrega y mensajería

| Término | Significado | Aparece en |
|---|---|---|
| **At-least-once** | Garantía de que cada mensaje se entrega **una o más** veces. Nunca se pierde, pero puede duplicarse. | [estado](estado.md), [decisiones (servicio)](servicio/decisiones.md), [garantias-de-entrega](servicio/garantias-de-entrega.md) |
| **Exactly-once** | Cada mensaje produce efecto exactamente una vez. Requiere transacciones entre origen y destino; aquí se aproxima con at-least-once + idempotencia. | [decisiones (servicio)](servicio/decisiones.md), [garantias-de-entrega](servicio/garantias-de-entrega.md) |
| **Idempotencia** | Aplicar una operación varias veces deja el mismo resultado que aplicarla una. Permite reintentar sin miedo. | [estado](estado.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [topologia-nats](servicio/topologia-nats.md), [pruebas](operaciones/pruebas.md) |
| **Deduplicación (dedup)** | Detectar y descartar copias del mismo evento, aquí por `event_id`. | [estado](estado.md), [arquitectura](servicio/arquitectura.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md), [evolucion-del-esquema](operaciones/evolucion-del-esquema.md), [pruebas](operaciones/pruebas.md) |
| **Ack** | Confirmación al broker de que un mensaje está procesado; deja de reentregarse. | [estado](estado.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [memoria](servicio/memoria.md), [topologia-nats](servicio/topologia-nats.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [entorno-local](operaciones/entorno-local.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Nak** | Rechazo temporal: pide al broker que reentregue (opcionalmente con retraso). | [estado](estado.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **Term** | Rechazo definitivo: el broker no lo reentregará nunca. Para mensajes venenosos. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **+WIP (in progress)** | Señal de "sigo trabajando": reinicia el temporizador de *AckWait* sin confirmar. | [estado](estado.md) |
| **Redelivery (reentrega)** | El broker vuelve a enviar un mensaje no confirmado a tiempo. | [concurrencia](servicio/concurrencia.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [topologia-nats](servicio/topologia-nats.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Mensaje venenoso** | Mensaje que nunca podrá procesarse (JSON inválido, datos fuera de dominio). Reintentarlo es inútil. | [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **DLQ (dead-letter queue)** | Destino aparte para mensajes que no se pudieron procesar; se revisan o reprocesan después. | [estado](estado.md), [arquitectura](servicio/arquitectura.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [gateway](servicio/gateway.md), [memoria](servicio/memoria.md), [navegacion-del-codigo](servicio/navegacion-del-codigo.md), [topologia-nats](servicio/topologia-nats.md), [entorno-local](operaciones/entorno-local.md), [evolucion-del-esquema](operaciones/evolucion-del-esquema.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Replay / backfill** | Volver a consumir mensajes antiguos del stream, p. ej. tras corregir un bug. | [estado](estado.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [topologia-nats](servicio/topologia-nats.md), [evolucion-del-esquema](operaciones/evolucion-del-esquema.md) |
| **Frescura** | Tiempo entre que ocurre un evento y que es consultable. | [concurrencia](servicio/concurrencia.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Lag** | Mensajes pendientes de consumir; mide cuánto va por detrás el procesador. | [estado](estado.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [entorno-local](operaciones/entorno-local.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Tiempo de evento** | Cuándo ocurrió (`occurred_at`), frente al tiempo de procesamiento (cuándo llegó). | [estado](estado.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md), [entorno-local](operaciones/entorno-local.md), [observabilidad](operaciones/observabilidad.md) |
| **Sesionización** | Agrupar eventos de un usuario en sesiones según pausas de inactividad. | [topologia-nats](servicio/topologia-nats.md) |

## NATS JetStream

| Término | Significado | Aparece en |
|---|---|---|
| **Stream** | Registro persistente de mensajes, en disco y replicado. | [arquitectura](servicio/arquitectura.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [gateway](servicio/gateway.md), [navegacion-del-codigo](servicio/navegacion-del-codigo.md), [topologia-nats](servicio/topologia-nats.md), [entorno-local](operaciones/entorno-local.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Subject** | "Dirección" jerárquica de un mensaje (`events.user_behavior.content_viewed`); admite comodines (`events.>`). | [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [gateway](servicio/gateway.md), [navegacion-del-codigo](servicio/navegacion-del-codigo.md), [topologia-nats](servicio/topologia-nats.md) |
| **Retención `limits`** | Los mensajes se conservan hasta un límite de edad/tamaño, aunque se hayan consumido. | [topologia-nats](servicio/topologia-nats.md) |
| **Retención `workqueue`** | Cada mensaje se borra al confirmarse; impide el replay. | [topologia-nats](servicio/topologia-nats.md) |
| **Consumidor durable** | Consumidor con nombre cuyo progreso guarda el servidor; sobrevive a reinicios y lo comparten varios pods. | [concurrencia](servicio/concurrencia.md), [gateway](servicio/gateway.md), [topologia-nats](servicio/topologia-nats.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |
| **Consumidor pull** | El cliente pide lotes cuando tiene capacidad (frente a *push*, donde el servidor empuja). | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [topologia-nats](servicio/topologia-nats.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |
| **AckWait** | Tiempo máximo para confirmar un mensaje antes de que se reentregue. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [topologia-nats](servicio/topologia-nats.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [kubernetes](operaciones/kubernetes.md) |
| **MaxAckPending** | Máximo de mensajes entregados y sin confirmar, para todo el consumidor. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [memoria](servicio/memoria.md), [topologia-nats](servicio/topologia-nats.md), [observabilidad](operaciones/observabilidad.md) |
| **MaxDeliver** | Número máximo de entregas de un mensaje. | [decisiones (servicio)](servicio/decisiones.md), [topologia-nats](servicio/topologia-nats.md) |
| **BackOff** | Lista de retrasos entre reentregas sucesivas. Si se define, **sustituye a AckWait** en cada entrega. | [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [topologia-nats](servicio/topologia-nats.md) |
| **Duplicate window** | Ventana en la que JetStream descarta publicaciones con `Nats-Msg-Id` repetido. | [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [gateway](servicio/gateway.md), [topologia-nats](servicio/topologia-nats.md), [kubernetes](operaciones/kubernetes.md) |
| **Advisory** | Evento de sistema que publica JetStream (p. ej. "mensaje agotó MaxDeliver"). | — |
| **Subject mapping** | Reescritura de subjects en el servidor; permite repartir en particiones deterministas. | [topologia-nats](servicio/topologia-nats.md) |

## ClickHouse

| Término | Significado | Aparece en |
|---|---|---|
| **OLAP** | Procesamiento analítico: consultas que agregan muchas filas y pocas columnas. | [arquitectura](servicio/arquitectura.md), [decisiones (servicio)](servicio/decisiones.md), [tabla-events](servicio/tabla-events.md) |
| **Tabla ancha (one big table)** | Una única tabla con todas las columnas necesarias para analizar sin joins. | [estado](estado.md), [arquitectura](servicio/arquitectura.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [decisiones (servicio)](servicio/decisiones.md), [tabla-events](servicio/tabla-events.md) |
| **Desnormalización** | Copiar datos relacionados (contenido, contexto) en cada fila en vez de referenciarlos. | [decisiones (servicio)](servicio/decisiones.md), [tabla-events](servicio/tabla-events.md) |
| **Parte (part)** | Fichero inmutable que crea cada INSERT; el servidor las fusiona en segundo plano. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md) |
| **Merge (fusión)** | Proceso de fondo que combina partes; es cuando `ReplacingMergeTree` elimina duplicados. | [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md) |
| **Partición** | División física de la tabla (aquí, por día); las partes nunca se fusionan entre particiones. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [decisiones (servicio)](servicio/decisiones.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md) |
| **Clave de ordenación** | `ORDER BY` de la tabla: orden físico, índice primario y clave de deduplicación. | [estado](estado.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md) |
| **ReplacingMergeTree** | Motor que, al fusionar, conserva una sola fila por clave de ordenación (la de versión mayor). | [estado](estado.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [decisiones (servicio)](servicio/decisiones.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md) |
| **FINAL** | Modificador de consulta que aplica la deduplicación en el momento de leer. Más costoso. | [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md) |
| **insert_deduplication_token** | Identificador de un INSERT; si se repite dentro de la ventana, el servidor lo ignora. | [estado](estado.md), [garantias-de-entrega](servicio/garantias-de-entrega.md) |
| **LowCardinality** | Codificación por diccionario para columnas con pocos valores distintos. | [tabla-events](servicio/tabla-events.md) |
| **Serialización dispersa (sparse)** | Almacena solo los valores no por defecto de una columna mayoritariamente vacía. | [tabla-events](servicio/tabla-events.md) |
| **RowBinary** | Formato binario por filas con el que el cliente Rust envía los INSERT. | [memoria](servicio/memoria.md) |
| **Shard** | Porción horizontal de los datos en un clúster; cada shard guarda filas distintas. | [estado](estado.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |
| **Réplica** | Copia completa de un shard para tolerar fallos. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [tabla-events](servicio/tabla-events.md) |
| **Keeper** | Servicio de coordinación de ClickHouse (equivalente a ZooKeeper) para replicación. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |
| **Tabla Distributed** | Tabla lógica que reparte inserciones y consultas entre shards. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |
| **Vista materializada** | Transformación que se ejecuta en cada INSERT y escribe en otra tabla (p. ej. agregados). | [estado](estado.md), [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |
| **AggregatingMergeTree** | Motor que guarda estados de agregación parciales y los combina al fusionar. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |
| **TTL** | Regla de expiración: borra o mueve filas/particiones pasado un tiempo. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |
| **async_insert** | Modo en que el servidor agrupa muchos INSERT pequeños antes de escribir partes. | [clickhouse-a-escala](servicio/clickhouse-a-escala.md) |

## Concurrencia (Rust / tokio)

| Término | Significado | Aparece en |
|---|---|---|
| **Pipelining** | Encadenar etapas que trabajan a la vez sobre elementos distintos (llenar el lote 2 mientras se inserta el 1). | [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md) |
| **Tarea (task)** | Unidad ligera de trabajo asíncrono de tokio; miles por proceso sin coste de hilos. | [arquitectura](servicio/arquitectura.md), [concurrencia](servicio/concurrencia.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [memoria](servicio/memoria.md), [entorno-local](operaciones/entorno-local.md), [evolucion-del-esquema](operaciones/evolucion-del-esquema.md), [kubernetes](operaciones/kubernetes.md), [pruebas](operaciones/pruebas.md) |
| **Canal `mpsc` acotado** | Cola entre tareas con capacidad fija; si está llena, el emisor espera. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md), [memoria](servicio/memoria.md) |
| **Backpressure** | Propagación de la lentitud aguas arriba: el productor frena porque el consumidor no da abasto. | [concurrencia](servicio/concurrencia.md), [decisiones (servicio)](servicio/decisiones.md) |
| **Semaphore** | Contador de permisos que limita cuántas tareas hacen algo a la vez (aquí, K inserciones). | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [memoria](servicio/memoria.md) |
| **JoinSet** | Conjunto de tareas lanzadas que se pueden esperar juntas; reporta fallos y `panic`s. | [concurrencia](servicio/concurrencia.md) |
| **CancellationToken** | Señal compartida para pedir a todas las tareas que terminen ordenadamente. | [concurrencia](servicio/concurrencia.md) |
| **`select!`** | Espera a varias operaciones a la vez y actúa sobre la primera que termine. | — |
| **`spawn_blocking`** | Ejecuta trabajo CPU intensivo en un pool aparte para no bloquear el runtime. | — |
| **`Arc`** | Puntero con contador de referencias para compartir datos inmutables entre tareas sin copiarlos. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [garantias-de-entrega](servicio/garantias-de-entrega.md) |
| **`Send` / `Sync`** | Marcas de Rust: el tipo puede moverse / compartirse entre hilos con seguridad. | — |
| **Prefetch** | Pedir mensajes por adelantado para que la siguiente etapa no espere. | [concurrencia](servicio/concurrencia.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [memoria](servicio/memoria.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |

## Resiliencia

| Término | Significado | Aparece en |
|---|---|---|
| **Fallo transitorio** | Fallo que puede desaparecer reintentando (red, sobrecarga). | [decisiones (servicio)](servicio/decisiones.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [navegacion-del-codigo](servicio/navegacion-del-codigo.md), [topologia-nats](servicio/topologia-nats.md) |
| **Fallo permanente** | Fallo que se repetirá siempre con los mismos datos (formato, esquema). | [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [navegacion-del-codigo](servicio/navegacion-del-codigo.md) |
| **Backoff exponencial** | Esperar cada vez el doble entre reintentos (aquí 0,5 s, 1 s, 2 s…). | [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **Jitter** | Aleatoriedad añadida al backoff para que muchos clientes no reintenten a la vez. | [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **Circuit breaker** | Interruptor que, tras varios fallos seguidos, deja de llamar a la dependencia un tiempo. | [estado](estado.md), [arquitectura](servicio/arquitectura.md), [concurrencia](servicio/concurrencia.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [topologia-nats](servicio/topologia-nats.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **Half-open** | Estado del circuit breaker en el que deja pasar una prueba para ver si la dependencia volvió. | — |
| **Bisección** | Partir un lote rechazado en mitades sucesivas hasta aislar las filas culpables. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **Parada ordenada (graceful shutdown)** | Terminar el trabajo en curso antes de salir, sin perder ni duplicar más de lo necesario. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |

## Kubernetes y operación

| Término | Significado | Aparece en |
|---|---|---|
| **HPA** | Horizontal Pod Autoscaler: ajusta réplicas según CPU/memoria u otras métricas. | [estado](estado.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |
| **KEDA** | Autoescalado por eventos; aquí, según el lag del consumidor de JetStream. | [estado](estado.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |
| **QoS Guaranteed** | Clase de pod con `requests = limits`; es el último en ser desalojado. | [memoria](servicio/memoria.md), [kubernetes](operaciones/kubernetes.md) |
| **OOMKill** | El kernel mata el proceso por superar su límite de memoria. | [fallos-y-reintentos](servicio/fallos-y-reintentos.md) |
| **Liveness probe** | Comprobación de "el proceso está vivo"; si falla, Kubernetes reinicia el pod. | [estado](estado.md), [arquitectura](servicio/arquitectura.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [entorno-local](operaciones/entorno-local.md), [kubernetes](operaciones/kubernetes.md) |
| **Readiness probe** | Comprobación de "puede recibir trabajo"; si falla, se le deja de enviar tráfico. | [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [kubernetes](operaciones/kubernetes.md) |
| **SIGTERM** | Señal que envía Kubernetes antes de parar un pod; inicia la parada ordenada. | [concurrencia](servicio/concurrencia.md), [gateway](servicio/gateway.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md) |
| **Fragmentación** | Memoria libre partida en huecos pequeños inutilizables; crece en procesos de larga vida. | [memoria](servicio/memoria.md) |
| **Profiling** | Medir dónde se gasta CPU/memoria antes de optimizar. | [estado](estado.md), [concurrencia](servicio/concurrencia.md), [memoria](servicio/memoria.md) |

## Ingesta y pruebas de carga

| Término | Significado | Aparece en |
|---|---|---|
| **Gateway de ingesta** | Servicio HTTP que recibe eventos de los clientes y los publica en NATS (`event-gateway`). | [estado](estado.md), [arquitectura](servicio/arquitectura.md), [decisiones (servicio)](servicio/decisiones.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [gateway](servicio/gateway.md), [navegacion-del-codigo](servicio/navegacion-del-codigo.md), [topologia-nats](servicio/topologia-nats.md), [decisiones (operaciones)](operaciones/decisiones.md), [despliegue-y-escalado](operaciones/despliegue-y-escalado.md), [entorno-local](operaciones/entorno-local.md), [evolucion-del-esquema](operaciones/evolucion-del-esquema.md), [kubernetes](operaciones/kubernetes.md), [observabilidad](operaciones/observabilidad.md), [pruebas](operaciones/pruebas.md) |
| **202 Accepted** | Respuesta HTTP: la petición se aceptó para procesarse después (aquí, ya guardada en JetStream). | [gateway](servicio/gateway.md), [pruebas](operaciones/pruebas.md) |
| **Nats-Msg-Id** | Cabecera con la que JetStream reconoce y descarta publicaciones repetidas. | [estado](estado.md), [fallos-y-reintentos](servicio/fallos-y-reintentos.md), [garantias-de-entrega](servicio/garantias-de-entrega.md), [gateway](servicio/gateway.md), [kubernetes](operaciones/kubernetes.md) |
| **VU (virtual user)** | Usuario simulado de k6; ejecuta iteraciones del script en paralelo con los demás. | [pruebas](operaciones/pruebas.md) |
| **Arrival rate** | Ejecutor de k6 que fija las peticiones por segundo, independientemente de lo que tarden. | — |
| **Threshold** | Criterio de éxito de k6 (p. ej. `p(99)<200`); si no se cumple, la prueba falla. | [pruebas](operaciones/pruebas.md) |
| **Remote write** | Protocolo con el que k6 empuja métricas a Prometheus en vez de que Prometheus las recoja. | [kubernetes](operaciones/kubernetes.md), [pruebas](operaciones/pruebas.md) |
| **Conciliación** | Comprobar al final que todo lo aceptado está en ClickHouse o en la DLQ: ni pérdidas ni duplicados. | [estado](estado.md), [kubernetes](operaciones/kubernetes.md), [pruebas](operaciones/pruebas.md) |
