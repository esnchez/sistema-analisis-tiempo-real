# Sistema de Análisis en Tiempo Real para Medios de Comunicación

**Problema:** Las empresas de medios a menudo enfrentan desafíos para analizar y reaccionar a grandes volúmenes de datos de usuarios en tiempo real, lo que puede llevar a decisiones retrasadas y a una experiencia de usuario subóptima.

**Solución:** Construí un sistema de análisis en tiempo real para medios de comunicación utilizando flujos de datos en tiempo real, mejorando el tiempo de respuesta y la personalización del contenido usando Rust y ClickHouse.

**Impacto:** Procesó 2TB de datos en tiempo real diariamente, mejoró el tiempo de respuesta analítico de 10 minutos a 500ms.

**Tecnologías:** Rust, ClickHouse, Kubernetes, NATS, Temporal

**Tecnologías nuevas a aprender:** Rust, ClickHouse, NATS, Kubernetes


## Plan por fases

1. Fase 1: Diseñar la arquitectura del sistema y definir el esquema de datos
2. Fase 2: Implementar la ingesta de datos y la capa de almacenamiento en ClickHouse
3. Fase 3: Desarrollar el procesador de análisis en Rust y configurar flujos de datos con NATS
4. Fase 4: Desplegar en Kubernetes y medir métricas de rendimiento

## Explicación

Este proyecto tiene como objetivo construir un sistema de análisis en tiempo real para empresas de medios de comunicación, permitiendo a estas empresas procesar grandes volúmenes de datos de usuarios y reaccionar rápidamente a los cambios en el comportamiento del usuario. Utilizando tecnologías como Rust para el procesamiento de datos, ClickHouse para el almacenamiento y análisis, y NATS para la mensajería en tiempo real, este sistema está diseñado para ser altamente eficiente y escalable. Kubernetes se utiliza para orquestar los servicios y asegurar su disponibilidad. Los conceptos clave que se enseñan incluyen el procesamiento de flujos de datos en tiempo real, la orquestación de contenedores con Kubernetes, y el uso de ClickHouse para análisis de alto rendimiento. Además, se explorará cómo integrar servicios de mensajería con NATS y cómo Rust puede ser utilizado para construir servicios rápidos y seguros.

## Arquitectura

```
+-------------------+       +-------------------+
|   User Data      |       |   User Behavior   |
|   Sources        |       |   Sources         |
+-------------------+       +-------------------+
        |                           |
        v                           v
+-------------------+       +-------------------+
|    NATS          |<----->|   Rust Processor  |
|   Messaging      |       |   (Stream)        |
+-------------------+       +-------------------+
        |                            |
        v                            v
+-------------------+       +-------------------+
|    ClickHouse    |<----->|   Temporal        |
|    Database      |       |   Workflow        |
+-------------------+       +-------------------+
        |
        v
+-------------------+
|    Kubernetes    |
|   Orchestration  |
+-------------------+
```
Descripción de componentes: 
- **User Data Sources**: Fuentes de datos de usuarios que generan eventos. 
- **NATS Messaging**: Sistema de mensajería en tiempo real que distribuye eventos a los procesadores.
- **Rust Processor**: Servicio desarrollado en Rust que procesa flujos de datos en tiempo real.
- **ClickHouse Database**: Almacena y permite el análisis de grandes volúmenes de datos.
- **Temporal Workflow**: Gestiona la lógica de flujo de trabajo y orquestación de tareas complejas.
- **Kubernetes Orchestration**: Gestiona la implementación, escalabilidad y operación de contenedores.

## Entorno local (simulación cloud)

Para simular el entorno de cloud en local usamos un clúster propio de minikube (perfil `rtas`)
con todo dentro: NATS, ClickHouse, gateway, procesador, OpenTelemetry Collector, Prometheus y
Grafana. Guía completa con los comandos equivalentes: [docs/operaciones/kubernetes.md](docs/operaciones/kubernetes.md).

Todo se ejecuta con [Task](https://taskfile.dev) (`Taskfile.yml`); `task` lista las tareas. Las
del clúster usan `--context rtas` y nunca cambian el contexto activo de `kubectl`.

```bash
task cluster:start        # minikube `rtas` (con el arreglo para Docker lento en Windows)
task monitoring:install   # Prometheus + Grafana
task deploy               # imagen, carga en minikube, k8s/ y reinicio de gateway y procesador
task status               # pods
task grafana              # http://localhost:3000 (admin / admin), dashboard *RTAS · Pipeline en tiempo real*
```

Pruebas: `task test`, `task lint` y `task test:integration`; todas las tareas de prueba y sus
comandos equivalentes en [docs/operaciones/pruebas.md](docs/operaciones/pruebas.md#con-task).

Para desarrollo rápido sin Kubernetes: `task env:up`
([entorno con Docker Compose](docs/operaciones/entorno-local.md)).

## Despliegue a cloud

Para desplegar en un entorno de cloud real, seguiremos estos pasos usando Terraform para la infraestructura y EKS para Kubernetes.

1. Configura Terraform para desplegar un clúster de EKS:
```hcl
provider "aws" {
  region = "us-west-2"
}

module "eks" {
  source          = "terraform-aws-modules/eks/aws"
  cluster_name    = "media-analysis"
  cluster_version = "1.21"
  subnets         = ["subnet-abc123", "subnet-def456"]
  ...
}
```

2. Aplica la configuración de Terraform:
```bash
terraform init
terraform apply
```

3. Despliega los servicios en el clúster de EKS:
```bash
kubectl kustomize --load-restrictor LoadRestrictionsNone k8s | kubectl apply --server-side -f -
```

Checklist de puesta en producción:
- [ ] Verificar la conectividad de NATS y ClickHouse
- [ ] Configurar alertas de monitoreo
- [ ] Realizar pruebas de carga para validar la escalabilidad

## Pruebas de carga

Para realizar pruebas de carga utilizaremos k6.

1. Escenario de carga:
   - Ramp-up: 0 a 2000 usuarios concurrentes en 5 minutos
   - Duración total: 15 minutos

2. Ejecuta el script de carga (k6 publica eventos por HTTP en el gateway de ingesta). En el
   clúster de pruebas, como Job de Kubernetes, o con Docker Compose:
```bash
task load:cluster:external  # en el clúster, con k6 fuera de él (recomendado)
task load                   # en Docker Compose
```
   Otras variantes (Job de k6 dentro del clúster, humo, parámetros):
   [docs/operaciones/pruebas.md](docs/operaciones/pruebas.md#con-task).

3. Observa los resultados utilizando Prometheus y Grafana:
   - k6 envía sus métricas a Prometheus (remote write) junto con las del pipeline (lag de NATS, eventos guardados, frescura, DLQ)
   - Dashboard provisionado en Grafana *Prueba de carga · pipeline de extremo a extremo* ([dónde](docs/operaciones/kubernetes.md#acceder))
   - Al terminar, k6 concilia lo aceptado con lo guardado en ClickHouse y la DLQ

El script de prueba de carga se encuentra en `loadtest/k6.js`. Detalle de cada ejecución: [docs/operaciones/pruebas.md](docs/operaciones/pruebas.md). Conclusiones y extrapolación a estas métricas: [docs/README.md](docs/README.md#resultados-y-extrapolación-a-los-objetivos-del-readme).

## Métricas objetivo

| Métrica | Objetivo | Cómo medir |
| --- | --- | --- |
| QPS / RPS objetivo | 2.000 req/s sostenidos | Se mide utilizando k6 y verificando las métricas en Prometheus. |
| Latencia p50 / p95 / p99 | p99 < 200ms | Se mide con k6 y se visualiza en Grafana. |
| Tasa de error | < 0.1% | Monitorizando errores en los logs y métricas de k6. |
| Punto de saturación | Degradación a partir de ~3.500 req/s | Observando la latencia y tasa de errores durante las pruebas de carga. |

## Pruebas (unit / integración)

Para asegurar la calidad del sistema, se implementarán pruebas unitarias y de integración.

Ejemplo de prueba unitaria en Rust:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_processing() {
        let event = "{\"user_id\": \"123\", \"data\": \"example\"}";
        let result = process_event(event);
        assert!(result.is_ok());
    }
}
```

Para pruebas de integración, se simulará el flujo completo de datos desde NATS hasta ClickHouse, verificando que los datos se procesan y almacenan correctamente.

## Complejidades técnicas

### Alta disponibilidad y escalabilidad

**Por qué es difícil:** Es difícil asegurar que el sistema se mantenga disponible y escalable bajo cargas variables, especialmente con tráfico en tiempo real.

**Cómo se resuelve:** Se aborda mediante el uso de Kubernetes para orquestar múltiples instancias del procesador de datos y ClickHouse para manejar grandes volúmenes de datos eficientemente. Ejemplo: Configurar réplicas en el manifiesto de Kubernetes para escalar horizontalmente según la carga.

### Procesamiento en tiempo real

**Por qué es difícil:** Requiere manejar y procesar eventos a medida que llegan con mínima latencia.

**Cómo se resuelve:** Utilizando NATS para mensajería rápida y Rust para procesamiento eficiente. Ejemplo: Uso de suscripciones NATS para manejar eventos de usuario en tiempo real.

### Consistencia de datos

**Por qué es difícil:** Es crucial garantizar que los datos procesados y almacenados sean consistentes, especialmente en sistemas distribuidos.

**Cómo se resuelve:** Se asegura mediante el uso de transacciones en ClickHouse y manejo de errores en el procesador Rust. Ejemplo: Manejo de reintentos en caso de fallos durante la escritura en ClickHouse.


## Takeaways

- Implementé un sistema de procesamiento en tiempo real utilizando Rust y NATS, mejorando la personalización de contenido.
- Desplegué y gestioné servicios en Kubernetes, asegurando alta disponibilidad y escalabilidad.
- Optimizé consultas analíticas en ClickHouse para manejar 2TB de datos diarios.
- Desarrollé pruebas de carga para validar el rendimiento y la capacidad de escalado del sistema.
- Aprendí a integrar y orquestar flujos de datos en tiempo real con Temporal y NATS.

## Ruta de aprendizaje

### Rust

Rust es un lenguaje moderno que ofrece seguridad y rendimiento, ideal para aplicaciones de sistemas y procesamiento de datos en tiempo real.

1. Introducción a Rust - Aprender los conceptos básicos del lenguaje.
2. Proyecto: Construir un procesador de datos con Rust - Aplicar los conocimientos adquiridos en un entorno real.
3. Integración con NATS - Aprender a manejar mensajería en tiempo real.
4. Optimización de rendimiento - Mejorar el procesamiento y uso de memoria.


- The Rust Programming Language Book
- Rust by Example
- NATS.io Documentation

### ClickHouse

ClickHouse es una base de datos analítica de alto rendimiento, ideal para manejar grandes volúmenes de datos en tiempo real.

1. Fundamentos de ClickHouse - Comprender su arquitectura y características.
2. Proyecto: Configuración básica de ClickHouse - Implementar una instancia local y practicar consultas.
3. Optimización de consultas - Aprender a escribir consultas eficientes para análisis de datos.
4. Integración con sistemas de mensajería - Conectar ClickHouse con NATS y Rust para flujos de datos en tiempo real.


- Official ClickHouse Documentation
- ClickHouse Community Talks
- ClickHouse SQL Reference
