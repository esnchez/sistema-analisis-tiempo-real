# Pruebas de carga (k6)

Prueba de extremo a extremo según el README: clientes simulados con k6 → gateway → NATS →
procesador → ClickHouse, en Docker Compose o en Kubernetes ([entorno](kubernetes.md)).
Script: `loadtest/k6.js`.

```
 k6 ──HTTP──▶ gateway ──▶ NATS ──▶ processor ──▶ ClickHouse
  │  clients: rampa 0 → RATE req/s                          ▲
  │  observer: cada 5 s lee NATS (/jsz) y ClickHouse ───────┘
  └──remote write──▶ Prometheus ──▶ Grafana (dashboard "Prueba de carga")
```

## Ejecutar

```bash
docker compose --profile loadtest up -d --build           # pipeline + Prometheus + Grafana
docker compose --profile loadtest run --rm k6             # escenario del README (15 min)
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

## Qué se comprueba

| Comprobación | Cómo | Objetivo |
|---|---|---|
| Latencia del gateway | threshold `http_req_duration{scenario:clients}` | p99 < 200 ms (README) |
| Errores | threshold `http_req_failed{scenario:clients}` | < 0,1 % (README) |
| **Conciliación** | `handleSummary`: aceptados válidos = `uniqExact(event_id)` en ClickHouse · aceptados erróneos = mensajes nuevos en la DLQ | igualdad exacta: ni pérdidas ni duplicados |
| Lag, reentregas, frescura | escenario `observer` → métricas `pipeline_*` | lag que no crece; reentregas = 0 |

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
| README · 2 000 req/s × **1** evento | ~1 750 (2 000 en meseta) | ~1 700 | 19 ms / ✅ < 200 ms | ✅ 206 377 → 206 377 | 0 | ~2 s |
| 2 000 req/s × **10** eventos | **~660** (satura) | ~6 400 | 405 ms / ❌ | ✅ 1 178 654 → 1 178 654 | máx. 2 820 | ~2,4 s |

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
