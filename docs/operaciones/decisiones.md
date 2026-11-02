# Decisiones de operación

Registro breve del *qué* y el *porqué*. El detalle está en el documento enlazado. La numeración
es común a todo el proyecto; las decisiones del servicio están en
[servicio/decisiones.md](../servicio/decisiones.md).

| # | Decisión | Por qué | Detalle |
|---|---|---|---|
| 16 | k6 → gateway en vez de k6 → NATS | la extensión `xk6-nats` está abandonada (2023) y no funciona con k6 actual | [pruebas de carga](pruebas.md#por-qué-k6--gateway-y-no-k6--nats) |
