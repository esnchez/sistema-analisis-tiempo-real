# Pruebas y resultados

Todas las pruebas del sistema, cómo se ejecutan y qué dieron:

| Tipo | Qué cubre | Dónde |
|---|---|---|
| Unitarias | dominio, comandos y pipeline con fakes en memoria (sin servicios) | archivos `*_tests.rs` junto al código ([convenciones](../servicio/navegacion-del-codigo.md#convenciones)) |
| Integración | adaptadores contra NATS y ClickHouse reales, marcadas `#[ignore]` | `processor/tests/integration/` |
| Carga de extremo a extremo | k6 → gateway → NATS → procesador → ClickHouse, con conciliación | `loadtest/` |

Resumen y extrapolación a los objetivos del README: [docs/README](../README.md#resultados-y-extrapolación-a-los-objetivos-del-readme).

## Ejecución del 2026-11-05

Pipeline completo con Task sobre un clúster `rtas` recreado desde cero. El portátil tenía la CPU
muy cargada por otros procesos (antivirus, navegador, editor; 1,6 de 32 GB libres): k6 gastaba el
**doble de CPU por petición** que el 2026-11-02, así que las latencias y req/s absolutos son
peores; la CPU por unidad de trabajo se usa para la [extrapolación](../README.md#coste-medido-por-componente).

| Prueba | Entorno | Resultado |
|---|---|---|
| `task lint` · `task test` | portátil | ✅ sin avisos · 63 / 63 |
| `task test:integration` | Docker Compose | ✅ 5 / 5 |
| `task test:integration:cluster` | Kubernetes (`rtas`) | ✅ 5 / 5 |
| `task load` (README, 1 evento, 15 min) con el clúster en pausa | Docker Compose | ⚠️ 1 559 req/s de media, p95 266 ms ❌, **0 errores**, conciliación ✅ 1 364 510 → 1 364 510 · DLQ 13 909 → 13 909 |
| `task load:cluster BATCH=1` (README, 15 min) | Kubernetes | ⚠️ 1 063 req/s en meseta, p95 436 ms ❌, **0 errores**, conciliación ✅ 861 147 → 861 147 · DLQ 8 567 → 8 567 |
| `task load:cluster RATE=800 BATCH=10 RAMP=1m HOLD=5m` (eventos) | Kubernetes | ⚠️ 450 req/s = **4 400 eventos/s**, p95 1,6 s ❌, 0,8 % de `503`, conciliación **FALLO** con eventos *de más* (ver abajo) |
| mismo escenario, k6 con reintentos ante `503` | Kubernetes | ⚠️ 396 req/s = 3 840 eventos/s, p95 1,7 s ❌, **0 errores**, conciliación ✅ 1 401 596 → 1 401 596 · DLQ 14 054 → 14 054 |

Recursos medidos en Prometheus durante las mesetas (núcleos consumidos; espera de CPU entre paréntesis):

| Pod | README, 1 063 req/s × 1 | Eventos, 4 386 eventos/s |
|---|---|---|
| k6 | 1,77 (64 %) | 1,59 (64 %) |
| gateway ×2 | 0,21 cada uno (30 %) | 0,16 cada uno (25 %) |
| processor ×2 | 0,07 cada uno (14 %) · 15 MiB | 0,12 cada uno (20 %) · 43 MiB |
| nats | 0,29 (39 %) · 183 MiB | 0,47 (47 %) · 494 MiB |
| clickhouse | 0,44 (51 %) · 376 MiB | 0,62 (55 %) · 374 MiB |
| lote medio / INSERT p99 / frescura p99 | 1 058 filas / 2,3 s / 4,3 s | 4 714 filas / 2,5 s / 8,9 s |

ClickHouse tras las pruebas: 4,46 M filas, 228 MiB comprimidos (797 MiB sin comprimir, ratio 3,5),
**54 B por fila** frente a 524 B de JSON por evento. Consultas analíticas medidas en
[docs/README](../README.md#consultas-analíticas-objetivo-de-500-ms).

### k6 fuera del clúster (2026-11-05)

`task load:cluster:external`: k6 como contenedor de Docker en la red del nodo, contra NodePorts
temporales (`loadtest/external/nodeports.yaml`), para que no consuma la CPU del nodo. Escenario del
README con 1 evento por petición, 20:31–20:47 UTC, Chrome cerrado.

| Métrica | k6 dentro (mismo día) | **k6 fuera** |
|---|---|---|
| req/s en meseta | 1 063 | **1 938** (97 % del objetivo) |
| Iteraciones descartadas por k6 | 41 % | 4 % |
| p95 / p99 (k6) | 436 / 801 ms | **276 / 553 ms** ❌ |
| p95 / p99 (gateway) | 369 / 500 ms | 246 / 476 ms |
| Errores | 0 | **0** (1 461 306 peticiones, 28 420 reintentos de cliente) |
| Conciliación | ✅ | ✅ 1 417 822 → 1 417 822 · DLQ 14 385 → 14 385 |
| Lag máx. / reentregas / frescura p99 | 66 / 0 / 4,3 s | 102 / 0 / 3,0 s |

Recursos en la meseta:

| Qué | CPU | Espera de CPU | Memoria máx. |
|---|---|---|---|
| gateway ×2 | 0,43 cada uno | 21 % | 42 MiB |
| processor ×2 | 0,13 cada uno | 9 % | 45 MiB |
| nats | 0,57 | 26 % | 301 MiB |
| clickhouse | 0,53 | 21 % | 584 MiB |
| **pods del sistema** | **2,2** | | |
| contenedor del nodo `rtas` (`docker stats`, incluye Kubernetes, Prometheus y Grafana) | 3,0–4,7 (límite 4) | 36 % de periodos con *throttling* desde su creación | 3,7 GiB de 6 |
| contenedor de k6 | **3–5** | | |

El nodo y k6 juntos usan ~7,5–9 núcleos de los 8 hilos del portátil (i5-9400H, 4 núcleos
físicos): por eso no se subió minikube a 6 CPU ([conclusión](../README.md#por-qué-falla-el-p99-en-local)).

**La conciliación `FALLO` de la prueba de eventos**: 1 180 peticiones (0,8 %) recibieron `503`
porque NATS, sin CPU, no confirmó a tiempo (el gateway registró exactamente 1 180 avisos `no se
pudieron publicar los eventos` y errores de E/S del cliente NATS). Parte de esos lotes ya estaba en
JetStream, así que ClickHouse tenía 4 310 eventos válidos y la DLQ 36 **más** de los que k6 contaba
como aceptados: ni pérdidas ni duplicados, sino lotes que el cliente no reenvió pese a lo que pide
el [contrato del gateway](../servicio/gateway.md#reintentos-del-cliente). k6 ahora los reenvía
(`MAX_503_RETRIES`, ver [variables](#ejecutar-en-docker-compose)).

Una primera ejecución de `task load:cluster BATCH=1` se descartó porque Docker Compose seguía
arrancado ([Hallazgos](kubernetes.md#hallazgos)).

## Ejecución del 2026-11-02

| Prueba | Entorno | Resultado |
|---|---|---|
| Unitarias (`task test`) | portátil, sin servicios | ✅ 63 / 63 |
| Formato y clippy (`task lint`) | portátil | ✅ sin avisos |
| Integración (`--ignored`) | Docker Compose | ✅ 5 / 5: esquema de ClickHouse, reintento sin duplicar, gateway descarta reintentos, DLQ idempotente con reentrega, pipeline completo NATS → ClickHouse + DLQ |
| Carga README (2 000 req/s × 1 evento, 15 min) | Docker Compose | ✅ [resultados](#resultados-en-docker-compose-portátil-docker-con-8-vcpu--16-gb) |
| Integración (`task test:integration:cluster`) | Kubernetes (`rtas`) | ✅ 5 / 5 |
| Carga README (`task load:cluster BATCH=1`) | Kubernetes (`rtas`, 4 CPU, k6 dentro) | ⚠️ [resultados](#resultados-en-kubernetes-minikube-rtas-4-cpu--6-gib-k6-dentro-del-clúster) |

## Con Task

| Tarea | Prueba |
|---|---|
| `task test` | unitarias |
| `task lint` | `cargo fmt --check` + `cargo clippy -D warnings` |
| `task test:integration` | integración contra NATS y ClickHouse de Compose (los arranca) |
| `task cluster:forward` + `task test:integration:cluster` | integración contra el clúster (port-forwards en otra terminal; Compose parado) |
| `task load:smoke` | humo en Compose (~40 s) |
| `task load` | escenario del README en Compose con volúmenes limpios (`BATCH=1` por defecto) |
| `task load:cluster` | Job de k6 en `rtas`; `RATE`, `RAMP`, `HOLD` y `BATCH` como variables (`task load:cluster BATCH=1`) |
| `task load:cluster:external` | k6 fuera del clúster, en un contenedor de Docker contra NodePorts temporales: no consume la CPU del nodo (`BATCH=1` por defecto) |

Los comandos equivalentes están debajo.

## Unitarias e integración

En [Docker Compose](entorno-local.md):

```bash
cd processor
cargo test                                                           # unitarias (sin servicios)
CLICKHOUSE_PASSWORD=clickhouse cargo test --test integration -- --ignored   # requiere nats + clickhouse
```

En [Kubernetes](kubernetes.md), contra los servicios del clúster:

```bash
# Unitarias
cd processor && cargo test

# Integración contra los servicios del clúster (en otras dos terminales)
kubectl --context rtas -n rtas port-forward svc/nats 4222
kubectl --context rtas -n rtas port-forward svc/clickhouse 8123
CLICKHOUSE_PASSWORD=clickhouse cargo test --test integration -- --ignored
```

## Prueba de carga (k6)

Prueba de extremo a extremo según el README: clientes simulados con k6 → gateway → NATS →
procesador → ClickHouse, en Docker Compose o en Kubernetes ([entorno](kubernetes.md)).
Script: `loadtest/k6.js`.

```
 k6 ──HTTP──▶ gateway ──▶ NATS ──▶ processor ──▶ ClickHouse
  │  clients: rampa 0 → RATE req/s                          ▲
  │  observer: cada 5 s lee NATS (/jsz) y ClickHouse ───────┘
  └──remote write──▶ Prometheus ──▶ Grafana (dashboard "Prueba de carga · pipeline de extremo a extremo")
```

## Ejecutar en Docker Compose

```bash
docker compose --profile loadtest up -d --build           # pipeline + Prometheus + Grafana
docker compose --profile loadtest run --rm -e BATCH=1 k6 # escenario del README (15 min, 1 evento por petición)
docker compose --profile loadtest run --rm k6             # mismo escenario con 10 eventos por petición (satura en local)
docker compose --profile loadtest run --rm -e RATE=50 -e RAMP=10s -e HOLD=30s k6   # humo
```

Grafana: <http://localhost:3000> → carpeta *Pruebas de carga* (anónimo, solo lectura; `admin`/`admin` para editar).

| Variable | Defecto | Qué controla |
|---|---|---|
| `RATE` | `2000` | peticiones/s objetivo (README) |
| `RAMP` / `HOLD` | `5m` / `10m` | rampa y meseta (README: 15 min) |
| `MAX_VUS` | `2000` | usuarios concurrentes máximos (README) |
| `BATCH` | `10` | eventos por petición (SDK que agrupa) |
| `INVALID_RATE` | `0.01` | eventos erróneos u obsoletos (→ DLQ) |
| `RETRY_RATE` | `0.02` | peticiones que el cliente repite (no deben duplicar) |
| `MAX_503_RETRIES` | `3` | reenvíos de un lote respondido con `503` |
| `GATEWAY_URL`, `CLICKHOUSE_URL`, `CLICKHOUSE_PASSWORD`, `NATS_MONITOR_URL` | `localhost` (`:8080`, `:8123`, `:8222`); Compose y el Job los apuntan a los servicios | a dónde envía y de dónde concilia |
| `NATS_STREAM`, `NATS_CONSUMER`, `NATS_DLQ_STREAM` | `EVENTS`, `processor`, `EVENTS_DLQ` | qué stream y consumidor observa |

## Ejecutar en Kubernetes

Job de k6 dentro del clúster ([entorno](kubernetes.md)):

```bash
kubectl --context rtas -n rtas delete job k6 --ignore-not-found
kubectl kustomize --load-restrictor LoadRestrictionsNone loadtest/k8s \
  | kubectl --context rtas apply --server-side -f -
kubectl --context rtas -n rtas logs -f job/k6        # al final: conciliación OK / FALLO
```

El escenario por defecto (`RATE`, `RAMP`, `HOLD`, `BATCH`) está en `loadtest/k8s/k6-job.yaml`;
`task load:cluster` lo cambia con variables sin tocar el archivo. Si se cruza un threshold
(p. ej. p99 > 200 ms) k6 sale con error y el Job queda `Failed`; el resultado completo está en los
logs y en el dashboard de la prueba de carga ([Grafana](kubernetes.md#acceder)).

> [!CAUTION]
> No cambiar el clúster (rollouts, reinicios de ClickHouse) mientras corre una prueba: la
> conciliación cuenta los mensajes nuevos de la DLQ desde que empieza, así que el tráfico de
> otra ejecución o los reintentos de un ClickHouse reiniciándose la falsean.

## Qué se comprueba

| Comprobación | Cómo | Objetivo |
|---|---|---|
| Latencia del gateway | threshold `http_req_duration{scenario:clients}` | p99 < 200 ms (README) |
| Errores | threshold `http_req_failed{scenario:clients}` | < 0,1 % (README) |
| **Conciliación** | `handleSummary`: aceptados válidos = `uniqExact(event_id)` en ClickHouse · aceptados erróneos = mensajes nuevos en la DLQ | igualdad exacta (ni pérdidas ni duplicados), salvo los eventos sin confirmar de la fila siguiente |
| Checks de respuesta | threshold `checks{scenario:clients}` | > 99,9 % de respuestas `202` |
| Reintentos tras `503` | como un cliente real: espera `Retry-After` y reenvía el lote (hasta `MAX_503_RETRIES`, 3); métrica `retries_after_503` | lo no confirmado tras los reintentos cuenta como "sin confirmar" y la conciliación lo admite guardado o no |
| Lag, reentregas, frescura | escenario `observer` → métricas `k6_pipeline_*` en Prometheus | lag que no crece; reentregas = 0 |

> [!NOTE]
> El tráfico del observador se excluye de los thresholds (filtro `scenario:clients`).
> La conciliación se imprime al final (`OK` / `FALLO`) pero no cambia el código de salida de k6.

### Errores de cliente simulados

Cada uno lleva `event_id`, así que un reintento no lo duplica y acaba **una vez** en la DLQ.

| Error | Motivo en la DLQ |
|---|---|
| tipo desconocido (SDK más nuevo que el procesador) | `invalid_event` |
| falta `user_id` | `invalid_event` |
| fecha mal formada | `invalid_event` |
| reloj del dispositivo 30 días atrás | `out_of_time_window` |

## Resultados en Docker Compose (portátil, Docker con 8 vCPU / 16 GB)

| Escenario | req/s | eventos/s | p95 / p99 gateway | Conciliación | Lag NATS | Frescura p99 |
|---|---|---|---|---|---|---|
| humo · 50 req/s × 10 | 43 | 410 | 3,3 ms / — | ✅ 17 304 → 17 304 | 0 | ~2 s |
| README · 2 000 req/s × **1** evento (ejecución parcial) | ~1 750 (2 000 en meseta) | ~1 700 | 19 ms / ✅ < 200 ms | ✅ 206 377 → 206 377 | 0 | ~2 s |
| **README · 2 000 req/s × 1 evento · 15 min completos (2026-11-02)** | 1 689 de media; **2 000 sostenidos en la meseta** de 10 min | ~1 650 | 44,5 ms / ✅ < 200 ms | ✅ 1 477 567 → 1 477 567 · DLQ 14 800 → 14 800 | máx. 24 | ~2 s (p99 1 991 ms) |
| 2 000 req/s × **10** eventos | **~660** (satura) | ~6 400 | 405 ms / ❌ | ✅ 1 178 654 → 1 178 654 | máx. 2 820 | ~2,4 s |

Detalle de la ejecución completa del 2026-11-02 (`runId=mvztilxh`, 22:15–22:31 UTC):

| Métrica | Valor |
|---|---|
| Peticiones | 1 523 071 (1 492 544 iteraciones + 29 993 reintentos de cliente), **0 fallidas** |
| Checks `202` | 100 % (1 522 360 / 1 522 360) |
| Latencia del gateway | mediana 2,6 ms · p90 27 ms · p95 44,5 ms · máx. 410 ms; threshold p99 < 200 ms ✅ |
| Usuarios virtuales | máx. 428 de los 2 000 permitidos (con 1 evento por petición no hacen falta más) |
| Iteraciones descartadas | 7 628 (0,5 %): k6 no llegó a lanzarlas a tiempo; no son errores del sistema |
| NATS | sin ack máx. 4 469, pendientes máx. 24, **0 reentregas** |
| DLQ | 14 747 en el último muestreo del observador; 14 800 en la conciliación final |

> [!NOTE]
> k6 en Docker sobre Windows registra algunas duraciones mínimas negativas por saltos del reloj
> de la VM; no afectan a los percentiles.

Con 10 eventos por petición, k6 consume 3–5 de los 8 vCPU generando JSON, ClickHouse 2–3 y NATS 1;
el procesador apenas ~0,4 núcleos y sin lag sostenido. La frescura (~2 s) es la antigüedad máxima
del lote ([concurrencia](../servicio/concurrencia.md#parámetros-de-ajuste-en-este-orden)).

## Resultados en Kubernetes (minikube `rtas`, 4 CPU / 6 GiB, k6 dentro del clúster)

Job de k6 de `loadtest/k8s/` contra el Service `gateway` ([Kubernetes](kubernetes.md)).

| Escenario | req/s | eventos/s | p95 gateway (k6) | Conciliación | Lag NATS | Reentregas | Frescura p99 |
|---|---|---|---|---|---|---|---|
| humo · 50 req/s × 10 · antes de los ajustes | 42 | 400 | 739 ms ❌ | ✅ 37 115 → 37 115 | máx. 50 | 0 | ~2,8 s |
| humo · 50 req/s × 10 | 42 | 404 | **17 ms** ✅ | ✅ 37 076 → 37 076 | 0 | 0 | ~2 s |
| README · 2 000 req/s × 1 (15 min) | **~1 500 máx.** (satura) | ~1 600 máx. | 472 ms ❌ | ✅ 884 079 → 884 079 · DLQ 8 784 → 8 784 | máx. 115 | 0 | ~2,3 s (picos de 62 s) |
| **README · 2 000 req/s × 1 (15 min, 2026-11-02)** | **1 870 en meseta** (satura) | ~1 520 de media | 183 ms ✅ · p99 299 ms ❌ | ✅ 1 378 121 → 1 378 121 · DLQ 14 059 → 14 059 | máx. 123 | 0 | ~2,1 s (máx. 2,3 s) |

Detalle de la ejecución del 2026-11-02 (`runId=mvzvmoxf`, 23:15–23:30 UTC; recursos medidos en
Prometheus durante la meseta de 10 min):

| Pod | CPU (núcleos) | Espera de CPU (PSI) | Memoria máx. |
|---|---|---|---|
| k6 | 1,88 | 51 % | 869 MiB |
| nats (+ exporter) | 0,41 | 30 % | 288 MiB |
| clickhouse | 0,36 | 38 % | 417 MiB |
| gateway ×2 | 0,28 cada uno | 23 % | 29 MiB |
| processor ×2 | 0,09 cada uno | 10 % | **13 MiB** |

- 1 420 575 peticiones (27 712 reintentos de cliente), **0 fallidas**, checks 100 %.
- k6 descartó 107 358 iteraciones (7,4 %): no pudo lanzarlas a tiempo con la CPU saturada.
- Lotes de ~5 000 filas (p99): con ~900 eventos/s por pod el lote se corta por antigüedad (2 s), no
  por tamaño. `INSERT` p99 1,26 s, inflado por la espera de CPU de ClickHouse.
- Los pods suman ~3,4 de los 4 núcleos y todos esperan CPU: el límite es el nodo, no el
  pipeline.

"Antes de los ajustes": ClickHouse escribiendo sus tablas de log internas y NATS con 100m de
CPU (ver *Hallazgos* en [Kubernetes](kubernetes.md)). Entre ambas ejecuciones también se paró
otro clúster de minikube que compartía la CPU del host.

En la ejecución de ~1 500 req/s (tercera fila; 911 000 peticiones, 17 933 reintentos de cliente)
el nodo llegó al 98 % de CPU, ClickHouse gastó ~1,1 núcleos y la espera de CPU subió al 65–70 %;
aun así el procesador no falló ningún INSERT, el circuit breaker no se abrió y no hubo reentregas.

## Por qué k6 → gateway y no k6 → NATS

k6 no habla NATS de serie. La extensión comunitaria `xk6-nats` (último commit en 2023) se
compila con k6 2.x pero queda **inactiva** (depende de `go.k6.io/k6`, no de `/v2`) y no
compila con k6 1.8. El gateway HTTP es además la entrada real de los clientes, y k6 lo prueba
con su módulo HTTP nativo y sus métricas estándar.
