# Pruebas y resultados

Todas las pruebas del sistema, cómo se ejecutan y qué dieron:

| Tipo | Qué cubre | Dónde |
|---|---|---|
| Unitarias | dominio, comandos y pipeline con fakes en memoria (sin servicios) | archivos `*_tests.rs` junto al código ([convenciones](../servicio/navegacion-del-codigo.md#convenciones)) |
| Integración | adaptadores contra NATS y ClickHouse reales, marcadas `#[ignore]` | `processor/tests/integration/` |
| Carga de extremo a extremo | k6 → gateway → NATS → procesador → ClickHouse, con conciliación | `loadtest/` |

## Última ejecución completa (2026-11-02)

| Prueba | Entorno | Resultado |
|---|---|---|
| Unitarias (`task test`) | portátil, sin servicios | ✅ 63 / 63 |
| Formato y clippy (`task lint`) | portátil | ✅ sin avisos |
| Integración (`--ignored`) | Docker Compose | ✅ 5 / 5: esquema de ClickHouse, reintento sin duplicar, gateway descarta reintentos, DLQ idempotente con reentrega, pipeline completo NATS → ClickHouse + DLQ |
| Carga README (2 000 req/s × 1 evento, 15 min) | Docker Compose | ✅ ver [resultados](#resultados-en-docker-compose-portátil-docker-con-8-vcpu--16-gb) |
| Integración (`task test:integration:cluster`) | Kubernetes (`rtas`) | ✅ 5 / 5 |
| Carga README (`task load:cluster BATCH=1`) | Kubernetes (`rtas`, 4 CPU, k6 dentro) | ⚠️ conciliación exacta y 0 errores, pero p99 299 ms > 200 ms y 1 870 req/s en meseta: CPU del clúster saturada, ver [resultados](#resultados-en-kubernetes-minikube-rtas-4-cpu--6-gib-k6-dentro-del-clúster) |

## Con Task

| Tarea | Prueba |
|---|---|
| `task test` | unitarias |
| `task lint` | `cargo fmt --check` + `cargo clippy -D warnings` |
| `task test:integration` | integración contra NATS y ClickHouse de Compose (los arranca) |
| `task cluster:forward` + `task test:integration:cluster` | integración contra el clúster (port-forwards en otra terminal; Compose parado) |
| `task load:smoke` | humo en Compose |
| `task load` | escenario del README en Compose con volúmenes limpios (`BATCH=1` por defecto) |
| `task load:cluster` | Job de k6 en `rtas`; `RATE`, `RAMP`, `HOLD` y `BATCH` como variables (`task load:cluster BATCH=1`) |

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

El escenario (`RATE`, `RAMP`, `HOLD`, `BATCH`) se cambia en `loadtest/k8s/k6-job.yaml`.
Si se cruza un threshold (p. ej. p99 > 200 ms) k6 sale con error y el Job queda `Failed`;
el resultado completo está en los logs. Dashboard en Grafana: **Prueba de carga · pipeline de extremo a extremo**, en la carpeta *General* (la carpeta *Pruebas de carga* solo existe en Compose).

> [!CAUTION]
> No cambiar el clúster (rollouts, reinicios de ClickHouse) mientras corre una prueba: la
> conciliación cuenta los mensajes nuevos de la DLQ desde que empieza, así que el tráfico de
> otra ejecución o los reintentos de un ClickHouse reiniciándose la falsean.

## Qué se comprueba

| Comprobación | Cómo | Objetivo |
|---|---|---|
| Latencia del gateway | threshold `http_req_duration{scenario:clients}` | p99 < 200 ms (README) |
| Errores | threshold `http_req_failed{scenario:clients}` | < 0,1 % (README) |
| **Conciliación** | `handleSummary`: aceptados válidos = `uniqExact(event_id)` en ClickHouse · aceptados erróneos = mensajes nuevos en la DLQ | igualdad exacta: ni pérdidas ni duplicados |
| Checks de respuesta | threshold `checks{scenario:clients}` | > 99,9 % de respuestas `202` |
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

> [!IMPORTANT]
> **El límite en local es la CPU compartida, no el pipeline.** Con 10 eventos por petición,
> k6 consume 3–5 de los 8 vCPU generando JSON, ClickHouse 2–3 y NATS 1; el procesador apenas
> ~0,4 núcleos y sin lag sostenido. Para medir la capacidad real hay que ejecutar k6 en otra
> máquina (o k6 Cloud) y dar recursos propios a cada servicio.

> [!TIP]
> La frescura (~2 s) es la antigüedad máxima del lote (`PIPELINE_BATCH_AGE_MS`): baja ese
> valor si necesitas datos antes, a costa de lotes más pequeños.

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
  pipeline. El procesador usa 0,09 núcleos y 13 MiB, frente a los 768 Mi de su límite
  ([memoria](../servicio/memoria.md)).

"Antes de los ajustes": ClickHouse escribiendo sus tablas de log internas y NATS con 100m de
CPU (ver *Hallazgos* en [Kubernetes](kubernetes.md)). Entre ambas ejecuciones también se paró
otro clúster de minikube que compartía la CPU del host.

> [!IMPORTANT]
> **Con 2 000 req/s el límite vuelve a ser la CPU compartida, no el pipeline.** El nodo llegó
> al 98 %: k6 ~1,9 núcleos, ClickHouse ~1,1, NATS ~0,4, cada gateway ~0,26 y cada procesador
> solo ~0,1. Todos esperaron CPU (PSI: k6 70 %, ClickHouse 65 %, NATS 41 %, gateways 34 %).
> El procesador no falló ni un INSERT, el circuit breaker no se abrió y no hubo reentregas:
> la conciliación fue exacta con 911 000 peticiones y 17 933 reintentos de cliente.

Para medir la capacidad real: k6 fuera del clúster (otra máquina o k6 Cloud) contra el
gateway expuesto, o más CPU para el nodo (`minikube start --cpus`).

## Por qué k6 → gateway y no k6 → NATS

k6 no habla NATS de serie. La extensión comunitaria `xk6-nats` (último commit en 2023) se
compila con k6 2.x pero queda **inactiva** (depende de `go.k6.io/k6`, no de `/v2`) y no
compila con k6 1.8. El gateway HTTP es además la entrada real de los clientes, y k6 lo prueba
con su módulo HTTP nativo y sus métricas estándar.
