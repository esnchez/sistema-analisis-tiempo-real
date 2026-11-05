# Entorno de pruebas en Kubernetes (minikube)

Todo el sistema y su monitorización dentro de un clúster propio de minikube (perfil `rtas`),
separado de otros clústeres locales. Manifiestos en `k8s/`.

```
 namespace rtas                                        namespace monitoring
   gateway ×2 ──▶ nats ──▶ processor ×2 ──▶ clickhouse
      │                        │
      └──── OTLP ──▶ otel-collector ◀──┘
                          │ :8889 ───────────────────▶ Prometheus ──▶ Grafana
   nats-exporter :7777 (sidecar de nats) ────────────▶     ▲
   clickhouse    :9363 ──────────────────────────────▶     │
   k6 (Job)  ── remote write ──────────────────────────────┘
```

| Directorio | Contenido |
|---|---|
| `k8s/` | el sistema: NATS, ClickHouse, gateway, procesador, colector OTel y sus ServiceMonitors |
| `k8s/config/` | configuración de NATS, ClickHouse y el colector (montada como ConfigMaps) |
| `k8s/monitoring/` | valores del chart `kube-prometheus-stack`, alertas y dashboards (los incluye `k8s/`) |
| `k8s/monitoring/grafana/dashboards/` | dashboards (Kubernetes; Compose reutiliza el de la prueba de carga) |
| `loadtest/k8s/` | Job de k6 (script: `loadtest/k6.js`) |
| `loadtest/external/` | NodePorts temporales para k6 fuera del clúster (`task load:cluster:external`) |
| `loadtest/compose/` | Prometheus y Grafana de Docker Compose (no se usan en Kubernetes) |

## Puesta en marcha

Con Task: `task cluster:start`, `task monitoring:install` y `task deploy` (las tres usan
`--context rtas`; ver `task --list`). Los pasos equivalentes:

```bash
# 1. Clúster propio; --keep-context no cambia el contexto activo de kubectl
minikube start -p rtas --driver=docker --cpus=4 --memory=6144 --keep-context

# 2. Monitorización (la primera vez descarga todas las imágenes: de ahí el timeout)
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts --force-update
helm --kube-context rtas upgrade --install kube-prometheus-stack \
  prometheus-community/kube-prometheus-stack -n monitoring --create-namespace \
  -f k8s/monitoring/kube-prometheus-stack-values.yaml --version 92.10.1 --wait --timeout 15m

# 3. Imagen del gateway y el procesador
docker build -t rtas-processor:dev processor
minikube -p rtas image load --overwrite=true rtas-processor:dev

# 4. Sistema (lee el esquema SQL de processor/, fuera de k8s/)
kubectl kustomize --load-restrictor LoadRestrictionsNone k8s \
  | kubectl --context rtas apply --server-side -f -
```

Tras cambiar el código: repetir el paso 3 y
`kubectl --context rtas -n rtas rollout restart deploy/processor deploy/gateway`.

## Acceder

Con Task: `task grafana`, `task prometheus` y `task status`. Equivalentes:

```bash
kubectl --context rtas -n monitoring port-forward svc/kube-prometheus-stack-grafana 3000:80   # admin / admin
kubectl --context rtas -n monitoring port-forward svc/kube-prometheus-stack-prometheus 9090
kubectl --context rtas -n rtas port-forward svc/gateway 8080                                  # POST /v1/events
```

Si el puerto local está ocupado (p. ej. por otro clúster), usar otro: `3001:80`.

Dashboards en Grafana (carpeta *General*): **RTAS · Pipeline en tiempo real** (servicio) y
**Prueba de carga · pipeline de extremo a extremo** (k6).
Alertas: Prometheus → *Alerts* (sin Alertmanager en pruebas).

## Pruebas

Unitarias, integración contra los servicios del clúster y el Job de k6, con sus resultados:
[pruebas](pruebas.md#ejecutar-en-kubernetes).

## Monitorización

| Fuente | Cómo llega a Prometheus | Métricas |
|---|---|---|
| gateway, procesador | OTLP/gRPC → colector → `:8889` (ServiceMonitor) | ver [observabilidad](observabilidad.md) |
| NATS | sidecar `prometheus-nats-exporter` `:7777` | lag, acks pendientes, reentregas, tamaño de streams |
| ClickHouse | endpoint propio `:9363` | filas insertadas, partes por partición |
| pods | cAdvisor (kube-prometheus-stack) | CPU, memoria, espera de CPU (PSI) |
| k6 | remote write | peticiones, latencia, conciliación |

## Probes

| Servicio | Liveness | Readiness |
|---|---|---|
| processor | `exec`: latido de `HEARTBEAT_FILE` de < 30 s | ninguna: no recibe tráfico |
| gateway | `GET /health` | `GET /health` |
| nats | `/healthz` | `/healthz?js-enabled-only=true` |
| clickhouse | `/ping` | `/ping` |
| otel-collector | TCP `:4317` | TCP `:4317` |

Por qué cada probe es así (y por qué la liveness no depende de NATS ni ClickHouse):
[despliegue](despliegue-y-escalado.md#procesador).

## Recursos (clúster de 4 CPU / 6 GiB)

| Pod | Peticiones | Límite |
|---|---|---|
| processor ×2 | 500m · 768 Mi | 500m · 768 Mi (QoS *Guaranteed*) |
| gateway ×2 | 250m · 64 Mi | 256 Mi |
| clickhouse | 500m · 1 Gi | 2 CPU · 2 Gi |
| nats + exporter | 510m · 288 Mi | 576 Mi |
| otel-collector | 50m · 64 Mi | 256 Mi |
| k6 (Job) | 1 CPU · 256 Mi | 2 Gi |

> [!WARNING]
> Con el driver `docker`, el nodo anuncia la CPU y memoria de **toda la VM de Docker**
> (aquí 8 CPU / 16 GiB), no las de `--cpus`/`--memory` (4 / 6 GiB), que solo limitan el
> contenedor. El planificador cree tener el doble de CPU: que las peticiones "quepan" no
> garantiza nada cuando el nodo se satura. Por eso ClickHouse lleva límite de CPU.

## Hallazgos

Problemas encontrados al montar el entorno y cómo se resolvieron.

| Síntoma | Causa | Solución |
|---|---|---|
| `minikube start` falla con `DRV_UNSUPPORTED_OS: The driver 'docker' is not supported on windows/amd64` | el mensaje engaña: minikube da 20 s a `docker system info`, que en Windows tarda 20–50 s porque consulta cada plugin del CLI de Docker (1,5–3,7 s cada uno). `--force` no lo evita | lanzar `docker system info` una o dos veces justo antes (se vuelve más rápido) y repetir el `start` |
| Con un perfil ya creado, el mismo problema aparece como `The driver '' is not supported` (2026-11-02: `docker system info` tardaba 21–27 s y repetirlo no bastó) | igual que el anterior; minikube no llega a leer el driver del perfil | `task cluster:start` lo resuelve: si `minikube start` falla, arranca el contenedor del nodo con `docker start rtas` |
| Tras `docker start rtas`: API server sin respuesta (`EOF`) | sin `minikube start` nadie arranca `containerd` ni `kubelet` dentro del nodo | `docker exec rtas systemctl start containerd kubelet` (lo hace `task cluster:start`) |
| Tras arrancar el nodo a mano: CoreDNS sin `Ready`, procesador y gateway en `CrashLoopBackOff` (`Temporary failure in name resolution`), operador de Prometheus caído | falta `control-plane.minikube.internal` en `/etc/hosts` del nodo: kube-proxy no llega al API server y no crea las reglas de los Services (`10.96.0.1` rechaza conexiones) | añadir la entrada antes de arrancar `kubelet` (lo hace `task cluster:start`) y reiniciar los pods afectados. **No borrar el pod `storage-provisioner`**: es un pod suelto y no se recrea (`minikube -p rtas addons enable storage-provisioner`) |
| `minikube update-context` deja `rtas` como contexto activo | es su comportamiento | `task cluster:start` restaura el contexto anterior |
| Docker Desktop deja de responder (`500 Internal Server Error` en toda la API) tras una prueba de carga larga | motor de Docker Desktop colgado | reiniciar Docker Desktop |
| Crear el clúster desde cero falla: `prepare kic ssh: unable to execute icacls to set permissions` y luego `GUEST_NOT_FOUND` (2026-11-05) | justo después de crearse, `machines\rtas\id_rsa` queda bloqueado por otro proceso (el antivirus del equipo analizando una clave privada nueva); `icacls` sobre el mismo archivo funciona segundos después | `minikube delete -p rtas` y repetir el `start` |
| `task monitoring:install` en un clúster nuevo: `Error: context deadline exceeded` y release `failed` | `helm --wait` espera 5 min por defecto y la primera instalación descarga todas las imágenes | `--timeout 15m` en la tarea; si ya falló, repetirla (la release pasa a `deployed`) |
| Prueba de carga en el clúster con resultados mucho peores de lo esperado (2026-11-05) | `task load` deja Docker Compose arrancado y su pipeline, Prometheus y Grafana compiten por la CPU de la VM de Docker; además ocupan los puertos 4222, 8123, 8222 y 9090, y un `port-forward` al Prometheus del clúster falla en silencio | `task env:down` antes; `task load:cluster`, `task load:cluster:external` y `task test:integration:cluster` ahora lo comprueban |
| Primer `task deploy`: gateway y procesador con 5–6 reinicios | arrancan antes que NATS y salen con error al no conectar; Kubernetes los reinicia hasta que NATS está listo | ninguna: se estabilizan solos en 1–2 min |
| `minikube image build` falla: `failed to read dockerfile: open Dockerfile` | desde Windows envía un Dockerfile vacío (`transferring dockerfile: 2B`), con ruta relativa o absoluta | `docker build` en el host + `minikube image load` |
| NATS en `CrashLoopBackOff`: `flag provided but not defined: -max_file_store` | el tope de disco de JetStream no existe como opción de línea de comandos | `k8s/config/nats.conf` con `jetstream { max_file_store }` |
| Job de k6 en `CreateContainerConfigError: secret "clickhouse" not found` | kustomize añade un sufijo hash al Secret y la kustomization de `loadtest/k8s` no lo conoce | Secret con nombre fijo (`disableNameSuffixHash`) |
| Logs con códigos de color (`\x1b[2m…`) en `kubectl logs` | `tracing` coloreaba la salida también fuera de una terminal | colores solo si la salida es una terminal |
| Conciliación `FALLO`: 114 mensajes de más en la DLQ | un mensaje inválido reentregado se apartaba dos veces: la DLQ publicaba sin `Nats-Msg-Id` | `Nats-Msg-Id = <stream>:<secuencia>` del mensaje original + `duplicate_window` de la DLQ de 10 min (> *AckWait*). Prueba: `tests/integration/infrastructure/nats/dead_letter.rs` |
| Latencia del gateway p95 ≈ 0,7 s a solo 50 req/s (en Compose: 3 ms) | nodo al 99 % de CPU; NATS esperando CPU hasta el 36 % del tiempo (PSI). ClickHouse gastaba ~0,8 núcleos **sin carga** escribiendo sus tablas de log internas (3,3 M filas en `asynchronous_metric_log`, frente a 90 k eventos) | `k8s/config/clickhouse-system-logs.xml` desactiva esas tablas; ClickHouse con límite de 2 CPU; NATS con petición de 500m |

## Limpiar

Con Task: `task cluster:stop` (para el clúster y conserva todo) o `task cluster:clean` (borra el
namespace `rtas`, con confirmación). Equivalentes:

```bash
kubectl --context rtas delete namespace rtas   # sistema y datos (PVC)
minikube delete -p rtas                         # el clúster entero
```
