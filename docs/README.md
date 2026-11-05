# Documentación

Documentos cortos, uno por tema, agrupados en dos carpetas. Los comandos para empezar están en
el [README del proyecto](../README.md) y en `task` (lista todas las tareas del
[`Taskfile.yml`](../Taskfile.yml)).

```
docs/
├── estado.md       # qué está hecho y qué falta
├── glosario.md     # términos y documentos donde aparece cada uno
├── servicio/       # el código: arquitectura, ingesta, procesamiento, almacenamiento, decisiones
└── operaciones/    # ejecutarlo, desplegarlo, observarlo y probarlo, con los resultados de las pruebas
```

## Resultados y extrapolación a los objetivos del README

Resumen de las pruebas del 2026-11-02 y 2026-11-05; el detalle de cada ejecución está en
[pruebas](operaciones/pruebas.md) y el estado frente a cada objetivo en [estado](estado.md).

### Lo que demuestran las pruebas

| Pregunta | Respuesta | Evidencia |
|---|---|---|
| ¿Se pierden o duplican eventos? | **No**: conciliación exacta en 7 de 8 ejecuciones (hasta 1,48 M eventos), con reintentos de cliente y eventos inválidos. La otra (2026-11-05) dio `FALLO` por eventos **de más**, no de menos: lotes respondidos con `503` que JetStream sí había guardado en parte y que k6 no reenviaba. k6 ahora reintenta como pide el contrato | [pruebas](operaciones/pruebas.md#ejecución-del-2026-11-05); 5 pruebas de integración en Compose y en el clúster |
| ¿Aguanta 2 000 req/s con p99 < 200 ms y < 0,1 % de errores? | **Sí** cuando tiene CPU: Compose el 2026-11-02, 2 000 req/s sostenidos, p95 44,5 ms, 0 errores | [Compose](operaciones/pruebas.md#resultados-en-docker-compose-portátil-docker-con-8-vcpu--16-gb) |
| ¿Y en el clúster local? | **Casi, sin el p99**: con k6 **fuera** del clúster, **1 938 req/s en meseta** (97 % del objetivo), 0 errores, p95 276 ms y p99 553 ms. Con k6 dentro del nodo de 4 vCPU se quedaba en 1 870 (2026-11-02) y 1 063 req/s (2026-11-05) | [k6 fuera](operaciones/pruebas.md#k6-fuera-del-clúster-2026-11-05) · [k6 dentro](operaciones/pruebas.md#resultados-en-kubernetes-minikube-rtas-4-cpu--6-gib-k6-dentro-del-clúster) |
| ¿Dónde está el cuello de botella? | En la **CPU del portátil**, no en el pipeline ([por qué](#por-qué-falla-el-p99-en-local)) | `docker stats` y Prometheus durante la meseta |
| ¿Por qué cambian tanto los resultados entre días? | El portátil comparte sus 8 CPU con Windows; el 2026-11-05 el antivirus y otras aplicaciones dejaban ~la mitad de CPU efectiva (k6 gastaba el doble por petición) | [pruebas](operaciones/pruebas.md#ejecución-del-2026-11-05) |

Las cifras absolutas de latencia dependen del portátil. Para extrapolar se usa la **CPU consumida
por unidad de trabajo**, que mide el trabajo hecho y no el tiempo esperando.

### Coste medido por componente

Núcleos de CPU consumidos en las mesetas de 4 ejecuciones (2026-11-02 y 2026-11-05, con k6 dentro
y fuera del clúster). Se dimensiona con el extremo alto del rango.

| Componente | Por 1 000 req/s con 1 evento | Por 1 000 eventos/s en lotes grandes (10 por petición, ~4 700 filas por lote) | Memoria máx. por pod |
|---|---|---|---|
| gateway | 0,29 – 0,44 | 0,07 | 42 MiB |
| procesador | 0,10 – 0,14 | 0,05 | 45 MiB |
| NATS (JetStream, 1 réplica) | 0,22 – 0,29 | 0,10 | 494 MiB |
| ClickHouse | 0,19 – 0,43 | 0,14 | 584 MiB |
| **Total del sistema** | **0,80 – 1,25** | **0,36** | |
| k6 (generador, no es parte del sistema) | 1,0 – 1,7 | 0,35 | 870 MiB |

Otros datos medidos: evento medio de **524 B** de JSON; en ClickHouse **54 B por fila** comprimida
(ratio 3,5); frescura ~2 s ([la antigüedad máxima del lote](servicio/concurrencia.md#parámetros-de-ajuste-en-este-orden)).

> [!NOTE]
> Agrupar eventos abarata todo: con lotes de ClickHouse 5 veces mayores el coste por evento baja a
> la mitad o un tercio. Con más tráfico los lotes crecen solos (se cortan por tamaño antes que por
> antigüedad), así que la extrapolación a volúmenes altos usa la columna de lotes grandes.

### Por qué falla el p99 en local

Con k6 dentro del nodo, a 2 000 req/s el sistema necesita 1,6–2,5 núcleos y k6 otros 2,0–3,4:
**3,6–5,9 núcleos en un nodo de 4**. Sacar k6 del clúster (`task load:cluster:external`) llevó la
meseta de 1 063 a **1 938 req/s** el mismo día, pero no el p99: a esa carga los pods del sistema
usan solo 2,2 núcleos, pero el nodo (con Kubernetes, Prometheus y Grafana) llega a 3–4,7 y el
contenedor de k6 a 3–5, así que **entre los dos ocupan los 8 hilos** (4 núcleos físicos) del
portátil. El gateway tarda lo que tarda NATS en confirmar, y NATS espera CPU (PSI del gateway 21 %,
NATS 26 %).

**Dar 6 CPU a minikube no ayudaría de forma apreciable**, y no se aplicó: el límite de 4 se alcanza
en picos (36 % de periodos con *throttling*), pero el equipo no tiene hilos libres; lo que ganara el
clúster lo perdería k6, que también descarta iteraciones por falta de CPU. La memoria no limita
(3,7 de 6 GiB).

En un nodo **dedicado** de 4 vCPU (al 70 %) y k6 en otra máquina, el sistema daría para
**~2 250–3 500 req/s** con 1 evento por petición, coherente con la degradación "a partir de ~3 500
req/s" que prevé el README. Confirmarlo exige otra máquina: este portátil no tiene más CPU que dar.

### Extrapolación a los objetivos del README

Supuestos: pods de 1 vCPU al 70 % de uso como máximo (margen para picos), al menos 2 réplicas de
cada servicio sin estado, NATS en clúster de 3 nodos con streams R3 y ClickHouse con 2 réplicas.
El escalado se supone lineal por evento.

| Objetivo | Carga | gateway | procesador | NATS | ClickHouse | CPU del sistema |
|---|---|---|---|---|---|---|
| Escenario del README | 2 000 req/s × 1 evento | **2 pods** (0,9 núcleos) | **2 pods** (0,3) | 3 nodos de 1 vCPU (0,6) | 1 shard × 2 réplicas de 2 vCPU (0,9) | ~2,6 núcleos (medido: 2,2 a 1 938 req/s) |
| Saturación del README | 3 500 req/s × 1 evento | **3 pods** (1,5) | **2 pods** (0,5) | 3 nodos de 2 vCPU (1,0) | 1 shard × 2 réplicas de 4 vCPU (1,5) | ~4,5 núcleos |
| 2 TB/día | 23 MB/s = **~44 000 eventos/s** (~4 400 req/s × 10) | **5 pods** (3,2) | **4 pods** (2,4) | 3 nodos de 8 vCPU (4,4 por réplica) | 2 shards × 2 réplicas de 6 vCPU (6,2) | ~16 núcleos (+ réplicas) |

Para **2 TB/día** hay que cambiar además la configuración, no solo las réplicas:

| Qué | Hoy | Necesario | Por qué |
|---|---|---|---|
| Retención de JetStream (`max_age`) y disco de NATS | 72 h, `max_file_store` 8 GB, PVC 10 Gi | 6–24 h y **0,5–2 TB por nodo** | a 23 MB/s, 8 GB se llenan en **~6 min** y 72 h serían ~6 TB por réplica |
| `NATS_MAX_ACK_PENDING` | 500 000 | **≥ 1 000 000** | pods × filas por lote × (K + 3) = 4 × 50 000 × 5 |
| Almacenamiento de ClickHouse | PVC 10 Gi | **~205 GB/día por réplica** (≈ 6 TB con 30 días de retención) | 3 800 M filas/día × 54 B |
| Memoria del procesador | límite 768 Mi | se mantiene | lotes de 50 000 filas ≈ 200–400 MiB en el peor caso |
| Ritmo de INSERT | — | ~1 INSERT/s en la flota (4 pods × un lote cada ~4,5 s) | lo que recomienda ClickHouse |

### Consultas analíticas (objetivo de 500 ms)

Sobre 4,5 M filas, con ClickHouse limitado a 2 vCPU y esperando CPU la mitad del tiempo:

| Consulta | Tiempo |
|---|---|
| top 10 contenidos por vistas, hoy | 0,02 s |
| eventos por tipo en la última hora | 0,5 s |
| lectura media por sección y minuto | 0,8 s |
| usuarios únicos por país y plataforma | 2,1 s |
| recuento exacto sin duplicados (`FINAL`) | 3,9 s |

Las agregaciones simples cumplen; las de cardinalidad (`uniq`) y las exactas no, y con
~3 800 M filas/día crecerían con el volumen. Para 500 ms a esa escala hacen falta **vistas
materializadas** por minuto (pendiente en [estado](estado.md)) y leer de ellas, no de la tabla ancha.

### Qué no se ha medido

- El sistema con **CPU dedicada** y k6 en otra máquina: k6 ya corre fuera del clúster, pero en el
  mismo portátil. Es la prueba que confirmaría la extrapolación y el p99.
- El coste de la replicación R3 de NATS y de las réplicas de ClickHouse (se mide con 1 réplica).
- TLS y red real entre nodos.
- La línea base de "10 minutos" del README: no hay sistema anterior con el que comparar.

## Empieza aquí

| Documento | Responde a |
|---|---|
| [estado.md](estado.md) | ¿Qué está hecho y qué falta? |
| [Visión general](servicio/arquitectura.md) | ¿Qué servicios hay, cómo fluye un evento, qué capas y carpetas? |
| [glosario.md](glosario.md) | Términos usados en la documentación y dónde aparece cada uno |

## [servicio/](servicio): el código

Para entender o cambiar `processor/`. Orden de lectura sugerido:

| Área | Documento | Responde a |
|---|---|---|
| **Arquitectura** | [Visión general](servicio/arquitectura.md) | ¿Qué servicios hay, cómo fluye un evento, qué capas y carpetas? |
| | [Cómo navegar el código](servicio/navegacion-del-codigo.md) | ¿Dónde toco para cambiar X? Convenciones |
| | [Decisiones del servicio](servicio/decisiones.md) | ¿Por qué está hecho así? |
| **Ingesta** | [Gateway](servicio/gateway.md) | ¿Cómo entran los eventos? Contrato HTTP, por qué no valida |
| **Procesamiento** | [Garantías de entrega](servicio/garantias-de-entrega.md) | ¿Cómo se evita perder o duplicar eventos? |
| | [Topología de NATS](servicio/topologia-nats.md) | Streams, subjects, consumidor, DLQ |
| | [Concurrencia](servicio/concurrencia.md) | ¿Cómo procesa un pod muchos mensajes de forma segura? |
| | [Fallos y reintentos](servicio/fallos-y-reintentos.md) | ¿Qué pasa cuando algo falla? |
| | [Memoria](servicio/memoria.md) | ¿Cuánta memoria usa un pod y cómo se acota? |
| **Almacenamiento** | [Tabla `events`](servicio/tabla-events.md) | Diseño de la tabla ancha |
| | [ClickHouse a escala](servicio/clickhouse-a-escala.md) | Partes, sharding, réplicas, retención |

## [operaciones/](operaciones): ejecutar, desplegar, observar y probar

| Documento | Responde a |
|---|---|
| [Entorno local](operaciones/entorno-local.md) | Docker Compose, comandos, variables |
| [Kubernetes](operaciones/kubernetes.md) | Clúster de pruebas en minikube con Prometheus y Grafana; problemas encontrados |
| [Despliegue y escalado](operaciones/despliegue-y-escalado.md) | Réplicas, parada ordenada, por qué cada probe, seguridad y volúmenes |
| [Evolución del esquema](operaciones/evolucion-del-esquema.md) | Cambios de columnas y de eventos sin romper nada |
| [Observabilidad](operaciones/observabilidad.md) | Qué medir y dónde mirarlo |
| [Pruebas y resultados](operaciones/pruebas.md) | Unitarias, integración y carga con k6: cómo se ejecutan y qué dieron |
| [Decisiones de operación](operaciones/decisiones.md) | ¿Por qué se prueba y se opera así? |

Las decisiones están numeradas en orden cronológico para todo el proyecto y repartidas entre
las dos carpetas según su tema.
