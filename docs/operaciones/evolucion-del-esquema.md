# Evolución del esquema y de los eventos

| Situación | Riesgo | Regla |
|---|---|---|
| Nueva columna | inserciones rechazadas | 1) `ALTER TABLE … ADD COLUMN` con `DEFAULT` · 2) desplegar el procesador. Solo cambios aditivos |
| Nuevo tipo/variante de evento | el procesador viejo lo trata como inválido → DLQ | desplegar el procesador **antes** que los clientes; si no, reprocesar la DLQ |
| Cambio incompatible del JSON | rotura silenciosa | campo `schema_version` en el sobre del evento; convivencia de versiones |
| Replay histórico | duplicados masivos | seguro por diseño ([dedup en 4 capas](../servicio/garantias-de-entrega.md)); consumidor nuevo con `DeliverByStartTime` |

> [!NOTE]
> El gateway no valida, así que un cliente adelantado al procesador no pierde eventos: quedan
> en la DLQ (`invalid_event`) y se pueden reprocesar tras desplegar.

> [!TIP]
> En local, el esquema solo se aplica con el volumen vacío: `task env:reset`
> ([entorno local](entorno-local.md#comandos)).
