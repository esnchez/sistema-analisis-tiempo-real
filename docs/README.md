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
| [Despliegue y escalado](operaciones/despliegue-y-escalado.md) | Réplicas, parada ordenada, probes |
| [Evolución del esquema](operaciones/evolucion-del-esquema.md) | Cambios de columnas y de eventos sin romper nada |
| [Observabilidad](operaciones/observabilidad.md) | Qué medir y dónde mirarlo |
| [Pruebas y resultados](operaciones/pruebas.md) | Unitarias, integración y carga con k6: cómo se ejecutan y qué dieron |
| [Decisiones de operación](operaciones/decisiones.md) | ¿Por qué se prueba y se opera así? |

Las decisiones están numeradas en orden cronológico para todo el proyecto y repartidas entre
las dos carpetas según su tema.
