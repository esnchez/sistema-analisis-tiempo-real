-- Esquema de ClickHouse para el almacén analítico.
--
-- Patrón "one big table": una única tabla ancha y desnormalizada con todos los eventos.
-- Cada fila lleva copiados el contexto del cliente y los metadatos del contenido, de modo que
-- las consultas OLAP agregan y filtran sin joins. Las columnas que no aplican a un tipo de
-- evento quedan con su valor por defecto ('' / 0 / false / []), que ClickHouse almacena con
-- serialización dispersa (sparse) y apenas ocupa. Se evita `Nullable` salvo cuando el 0 o el
-- vacío serían ambiguos.
--
-- Escritura idempotente: NATS entrega al menos una vez, así que un mismo evento puede llegar
-- varias veces. `ReplacingMergeTree` colapsa las filas con la misma clave de ordenación
-- (que termina en `event_id`) al fusionar partes, quedándose con la de `ingested_at` mayor.
-- La deduplicación es eventual: para un recuento exacto usar `FINAL` o `uniqExact(event_id)`.
--
-- La correspondencia con el código está en `src/infrastructure/clickhouse/row.rs` (`EventRow`).

CREATE TABLE IF NOT EXISTS events
(
    -- Identidad y tiempo
    event_id                    UUID,
    -- Tipo con el prefijo de su fuente, p. ej. `user_behavior.content_viewed`.
    event_type                  LowCardinality(String),
    user_id                     String,
    occurred_at                 DateTime64(3, 'UTC'),
    -- Momento en que el procesador escribió la fila. Versión para `ReplacingMergeTree`.
    ingested_at                 DateTime64(3, 'UTC'),

    -- Contexto del cliente (vacío en eventos de datos de usuario)
    session_id                  String,
    authenticated               Bool,
    platform                    LowCardinality(String),
    device_type                 LowCardinality(String),
    app_version                 LowCardinality(String),
    -- País deducido de la IP del cliente.
    country                     LowCardinality(String),

    -- Contenido, desnormalizado desde el CMS
    content_id                  String,
    content_type                LowCardinality(String),
    content_section             LowCardinality(String),
    content_author_id           LowCardinality(String),
    content_published_at        Nullable(DateTime64(3, 'UTC')),
    content_paywalled           Bool,

    -- Adquisición: visitas y registros
    traffic_source              LowCardinality(String),
    referrer                    String,
    campaign                    LowCardinality(String),

    -- Lectura (`content_engaged`)
    engaged_secs                UInt32,
    scroll_depth                UInt8,

    -- Reproducción (`playback_state_changed`)
    playback_action             LowCardinality(String),
    position_secs               UInt32,
    -- NULL en directos.
    duration_secs               Nullable(UInt32),
    bitrate_kbps                Nullable(UInt32),

    -- Compartir, búsqueda y recomendaciones
    share_channel               LowCardinality(String),
    search_query                String,
    search_results_count        UInt32,
    recommendation_module       LowCardinality(String),
    recommended_content_ids     Array(String),
    recommendation_position     UInt16,

    -- Publicidad (`ad_interacted`)
    ad_id                       String,
    ad_placement                LowCardinality(String),
    ad_action                   LowCardinality(String),

    -- Perfil (`registered`, `profile_updated`). Vacío = sin cambios.
    signup_method               LowCardinality(String),
    -- País declarado por el usuario, distinto de `country`.
    profile_country             LowCardinality(String),
    language                    LowCardinality(String),
    age_range                   LowCardinality(String),
    interests                   Array(LowCardinality(String)),

    -- Suscripción (`subscription_changed`)
    subscription_change         LowCardinality(String),
    subscription_plan           LowCardinality(String),
    previous_subscription_plan  LowCardinality(String),
    billing_period              LowCardinality(String),
    -- NULL en planes gratuitos.
    price_amount_minor          Nullable(UInt64),
    price_currency              LowCardinality(String),
    trial                       Bool,

    -- Consentimientos (`consent_updated`)
    consent_analytics           Bool,
    consent_personalization     Bool,
    consent_advertising         Bool,

    -- Newsletters (`newsletter_subscription_changed`)
    newsletter                  LowCardinality(String),
    newsletter_subscribed       Bool,

    -- Índices de salto para búsquedas puntuales fuera de la clave de ordenación.
    INDEX idx_content_id content_id TYPE bloom_filter GRANULARITY 4,
    INDEX idx_session_id session_id TYPE bloom_filter GRANULARITY 4
)
ENGINE = ReplacingMergeTree(ingested_at)
-- Particiones diarias: con ~2 TB/día permiten borrar o mover días enteros.
PARTITION BY toDate(occurred_at)
-- Las consultas típicas filtran por tipo de evento y rango temporal. Termina en `event_id`
-- para que la deduplicación de `ReplacingMergeTree` actúe por evento.
ORDER BY (event_type, toStartOfHour(occurred_at), user_id, event_id)
-- Recuerda los tokens de los últimos INSERT para descartar el reintento de un lote ya escrito
-- (`insert_deduplication_token`). En tablas no replicadas viene desactivado (0); las
-- `Replicated*` usan `replicated_deduplication_window`, activo por defecto.
SETTINGS non_replicated_deduplication_window = 1000;
