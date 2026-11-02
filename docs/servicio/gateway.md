# Gateway de ingesta (`event-gateway`)

Punto de entrada de los eventos de los clientes: HTTP → NATS JetStream.
Código: `src/bin/event-gateway.rs` · `infrastructure/http/` · `infrastructure/nats/publisher.rs` ·
comando `PublishEvents`.

```
cliente ──POST /v1/events──▶ gateway ──publish (espera confirmación)──▶ stream EVENTS ──▶ procesador
         [evento, evento…]      │ Nats-Msg-Id = event_id
                                │ subject  = events.<source>.<type> | events.unknown
                                ▼
         202 {"accepted": n}  cuando JetStream los ha guardado
```

## Contrato

| Petición | Respuesta | Significado |
|---|---|---|
| `POST /v1/events` con un array JSON (o un único valor) | `202 {"accepted": n}` | publicados y guardados de forma durable |
| `POST /v1/events` con `[]` | `202 {"accepted": 0}` | nada que publicar |
| cuerpo que no es JSON | `400` | nada publicado |
| más de 500 eventos o cuerpo > 1 MiB | `413` | nada publicado |
| JetStream no confirma | `503` + `Retry-After: 1` | **reintentar el lote completo** |
| `GET /health` | `200` | el proceso está vivo |

Los errores llevan el cuerpo `{"error": "<mensaje>"}`.

## Tolerante a propósito

> [!IMPORTANT]
> El gateway **no valida el contenido** de los eventos. Simula la entrada de clientes reales
> (web, apps, SDKs desactualizados) que envían eventos erróneos u obsoletos. Todo se publica;
> la validación vive en un único sitio, el procesador, que aparta lo inválido en la DLQ.

| Lo que sí hace | Para qué |
|---|---|
| leer `event_id` si es texto no vacío, de ≤ 128 caracteres y sin caracteres de control | `Nats-Msg-Id`: JetStream descarta reintentos del cliente |
| leer `source` + `event.type` si ambos cumplen `[a-z0-9_]{1,64}` | subject `events.<source>.<type>`; si no, `events.unknown` |
| conservar el JSON byte a byte | el procesador y la DLQ reciben exactamente lo enviado |
| limitar tamaño y número de eventos | proteger memoria y respetar el `max_payload` de NATS |

## Reintentos del cliente

Un `503` puede llegar con parte del lote ya publicado. El cliente reintenta **todo**: los ya
guardados se descartan por `Nats-Msg-Id` dentro de la `duplicate_window` (2 min).

## Configuración

| Variable | Defecto |
|---|---|
| `GATEWAY_ADDR` | `0.0.0.0:8080` |
| `GATEWAY_MAX_BODY_BYTES` | `1048576` |
| `GATEWAY_MAX_EVENTS` | `500` |
| `NATS_URL` y resto de `NATS_*` | ver [topología](topologia-nats.md) |

## Parada

Al recibir SIGTERM deja de aceptar conexiones y termina las peticiones en curso
([despliegue](../operaciones/despliegue-y-escalado.md)).

## ¿Por qué un servicio aparte del procesador?

Misma imagen y mismo crate, pero **distinto despliegue**: escala con el tráfico de clientes
(conexiones HTTP), no con la capacidad de escritura de ClickHouse. NATS desacopla ambos lados.
