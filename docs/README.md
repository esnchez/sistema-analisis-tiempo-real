# Documentación

Documentos cortos, uno por tema. Empieza por la **visión general**; los términos están en el
[glosario](glossary.md) y el avance en [estado](status.md).

| Área | Documento | Responde a |
|---|---|---|
| **Arquitectura** | [Visión general](architecture/overview.md) | ¿Qué servicios hay, cómo fluye un evento, qué capas y carpetas? |
| | [Decisiones clave](architecture/decisions.md) | ¿Por qué está hecho así? |
| | [Cómo navegar el código](architecture/code-navigation.md) | ¿Dónde toco para cambiar X? Convenciones |
| **Ingesta** | [Gateway](ingestion/gateway.md) | ¿Cómo entran los eventos? Contrato HTTP, por qué no valida |
| **Procesamiento** | [Garantías de entrega](processing/delivery-guarantees.md) | ¿Cómo se evita perder o duplicar eventos? |
| | [Topología de NATS](processing/nats-topology.md) | Streams, subjects, consumidor, DLQ |
| | [Concurrencia](processing/concurrency.md) | ¿Cómo procesa un pod muchos mensajes de forma segura? |
| | [Fallos y reintentos](processing/failure-handling.md) | ¿Qué pasa cuando algo falla? |
| | [Memoria](processing/memory.md) | ¿Cuánta memoria usa un pod y cómo se acota? |
| **Almacenamiento** | [Tabla `events`](storage/clickhouse-table.md) | Diseño de la tabla ancha |
| | [ClickHouse a escala](storage/clickhouse-at-scale.md) | Partes, sharding, réplicas, retención |
| **Operación** | [Entorno local](operations/local-environment.md) | Docker Compose, comandos, variables |
| | [Pruebas de carga](operations/load-testing.md) | k6, Grafana, conciliación |
| | [Kubernetes](operations/kubernetes.md) | Clúster de pruebas en minikube con Prometheus y Grafana |
| | [Despliegue y escalado](operations/deployment-and-scaling.md) | Réplicas, parada ordenada, probes |
| | [Evolución del esquema](operations/schema-evolution.md) | Cambios de columnas y de eventos sin romper nada |
| | [Observabilidad](operations/observability.md) | Qué medir y dónde mirarlo |
| | [Estado](status.md) | Qué está hecho y qué falta |
| | [Glosario](glossary.md) | Términos |
