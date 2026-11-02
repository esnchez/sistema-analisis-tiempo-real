# Imagen única con los dos binarios del crate; cada servicio elige el suyo con `command`:
#   data-processor  consume NATS → ClickHouse
#   event-gateway   HTTP → NATS
#
# Multi-etapa: se compila con la toolchain completa y se ejecuta sobre una Debian mínima
# de la misma versión (trixie), para que coincida la glibc.

FROM rust:1.99-trixie AS build
WORKDIR /usr/src/app
COPY . .
# Cachés de BuildKit: las dependencias no se recompilan si solo cambia el código.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release --locked --bins \
    && mkdir -p /out \
    && cp target/release/data-processor target/release/event-gateway /out/

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /out/ /usr/local/bin/
# Sin privilegios de root.
USER nobody
CMD ["data-processor"]
