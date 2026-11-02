// Prueba de carga de extremo a extremo: clientes → gateway HTTP → NATS → procesador → ClickHouse.
//
// Escenarios:
//   clients   clientes que publican lotes de eventos (rampa 0 → RATE peticiones/s y meseta).
//   observer  cada 5 s mide el pipeline (lag de NATS, filas guardadas, frescura, DLQ) y lo
//             publica como métricas de k6, para verlo todo junto en Grafana.
// Al terminar, `teardown` espera a que el procesador vacíe la cola y `handleSummary`
// concilia: eventos aceptados por el gateway = eventos en ClickHouse + eventos en la DLQ.
//
// Uso (ver docs/operaciones/pruebas.md):
//   docker compose --profile loadtest run --rm k6                         # escenario del README
//   docker compose --profile loadtest run --rm -e RATE=50 -e RAMP=10s -e HOLD=20s k6   # humo

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Gauge } from 'k6/metrics';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.0.2/index.js';

// ── Parámetros ─────────────────────────────────────────────────────────────
const GATEWAY_URL = __ENV.GATEWAY_URL || 'http://localhost:8080';
const CLICKHOUSE_URL = __ENV.CLICKHOUSE_URL || 'http://localhost:8123';
const CLICKHOUSE_PASSWORD = __ENV.CLICKHOUSE_PASSWORD || 'clickhouse';
const NATS_MONITOR_URL = __ENV.NATS_MONITOR_URL || 'http://localhost:8222';
const NATS_STREAM = __ENV.NATS_STREAM || 'EVENTS';
const NATS_CONSUMER = __ENV.NATS_CONSUMER || 'processor';
const NATS_DLQ_STREAM = __ENV.NATS_DLQ_STREAM || 'EVENTS_DLQ';

const RATE = Number(__ENV.RATE || 2000); // peticiones/s objetivo (README: 2.000 req/s)
const RAMP = __ENV.RAMP || '5m'; // README: rampa de 5 min
const HOLD = __ENV.HOLD || '10m'; // README: 15 min en total
const MAX_VUS = Number(__ENV.MAX_VUS || 2000); // README: 2.000 usuarios concurrentes
const BATCH = Number(__ENV.BATCH || 10); // eventos por petición (SDK que agrupa)
const INVALID_RATE = Number(__ENV.INVALID_RATE || 0.01); // eventos erróneos u obsoletos
const RETRY_RATE = Number(__ENV.RETRY_RATE || 0.02); // peticiones que el cliente repite

// ── Opciones ───────────────────────────────────────────────────────────────
export const options = {
  scenarios: {
    clients: {
      executor: 'ramping-arrival-rate',
      startRate: 0,
      timeUnit: '1s',
      preAllocatedVUs: Math.min(100, MAX_VUS),
      maxVUs: MAX_VUS,
      stages: [
        { duration: RAMP, target: RATE },
        { duration: HOLD, target: RATE },
      ],
      exec: 'publish',
    },
    observer: {
      executor: 'constant-vus',
      vus: 1,
      duration: addDurations(RAMP, HOLD),
      exec: 'observe',
    },
  },
  // Objetivos del README, solo sobre el tráfico de clientes (no el del observador).
  thresholds: {
    'http_req_duration{scenario:clients}': ['p(99)<200'],
    'http_req_failed{scenario:clients}': ['rate<0.001'],
    'checks{scenario:clients}': ['rate>0.999'],
  },
  teardownTimeout: '5m',
};

// ── Métricas propias ───────────────────────────────────────────────────────
const sentValid = new Counter('events_sent_valid');
const sentInvalid = new Counter('events_sent_invalid');
const clientRetries = new Counter('client_retries');
const natsPending = new Gauge('pipeline_nats_pending');
const natsAckPending = new Gauge('pipeline_nats_ack_pending');
const natsRedelivered = new Gauge('pipeline_nats_redelivered');
const storedEvents = new Gauge('pipeline_stored_events');
const deadLetters = new Gauge('pipeline_dead_letters');
const freshnessP99 = new Gauge('pipeline_freshness_p99_ms');

// ── Ciclo de vida ──────────────────────────────────────────────────────────
export function setup() {
  const health = http.get(`${GATEWAY_URL}/health`);
  if (health.status !== 200) {
    throw new Error(`gateway no disponible en ${GATEWAY_URL}: ${health.status}`);
  }
  // Identifica los eventos de esta ejecución (user_id = k6-<runId>-...).
  const runId = Date.now().toString(36);
  console.log(`runId=${runId}`);
  return { runId, dlqStart: dlqMessages() };
}

export function publish(data) {
  const events = [];
  let valid = 0;
  for (let i = 0; i < BATCH; i++) {
    if (Math.random() < INVALID_RATE) {
      events.push(invalidEvent(data.runId));
    } else {
      events.push(validEvent(data.runId));
      valid++;
    }
  }
  const body = JSON.stringify(events);
  const params = { headers: { 'Content-Type': 'application/json' }, tags: { name: 'POST /v1/events' } };

  const res = http.post(`${GATEWAY_URL}/v1/events`, body, params);
  if (check(res, { 'aceptado (202)': (r) => r.status === 202 })) {
    sentValid.add(valid);
    sentInvalid.add(events.length - valid);
  }
  // Reintento del cliente (p. ej. tras un timeout): mismo cuerpo, mismos event_id.
  // No debe duplicar nada: JetStream lo descarta por `Nats-Msg-Id`.
  if (Math.random() < RETRY_RATE) {
    clientRetries.add(1);
    const retry = http.post(`${GATEWAY_URL}/v1/events`, body, params);
    check(retry, { 'reintento aceptado (202)': (r) => r.status === 202 });
  }
}

export function observe(data) {
  const consumer = consumerInfo();
  if (consumer) {
    natsPending.add(consumer.num_pending);
    natsAckPending.add(consumer.num_ack_pending);
    natsRedelivered.add(consumer.num_redelivered);
  }
  deadLetters.add(dlqMessages() - data.dlqStart);
  // `count()` y no `uniqExact`: barato aunque la ejecución crezca (el exacto, al final).
  storedEvents.add(storedCount(data.runId, 'count()'));
  const freshness = clickhouse(
    `SELECT quantileOrNull(0.99)(dateDiff('millisecond', occurred_at, ingested_at)) AS v
     FROM events
     WHERE startsWith(user_id, 'k6-${data.runId}-') AND ingested_at > now64(3) - INTERVAL 10 SECOND`,
  );
  if (freshness !== null && freshness.v !== null) {
    freshnessP99.add(Number(freshness.v));
  }
  sleep(5);
}

// Espera a que el procesador vacíe la cola: nada pendiente ni sin confirmar.
export function teardown() {
  const deadline = Date.now() + 4 * 60 * 1000;
  while (Date.now() < deadline) {
    const consumer = consumerInfo();
    if (consumer && consumer.num_pending === 0 && consumer.num_ack_pending === 0) {
      return;
    }
    sleep(2);
  }
  console.warn('el procesador no vació la cola a tiempo: la conciliación fallará');
}

// Conciliación: todo evento aceptado está en ClickHouse (válidos) o en la DLQ (erróneos).
export function handleSummary(data) {
  const { runId, dlqStart } = data.setup_data;
  const metric = (name) => (data.metrics[name] ? data.metrics[name].values.count : 0);
  const expectedStored = metric('events_sent_valid');
  const expectedDead = metric('events_sent_invalid');
  const stored = storedCount(runId);
  const dead = dlqMessages() - dlqStart;
  const ok = stored === expectedStored && dead === expectedDead;

  const report = `
  conciliación (runId=${runId}) ............ ${ok ? 'OK' : 'FALLO'}
    válidos aceptados → en ClickHouse ...... ${expectedStored} → ${stored}
    erróneos aceptados → en la DLQ ......... ${expectedDead} → ${dead}
    reintentos de cliente (sin duplicar) ... ${metric('client_retries')}
`;
  return { stdout: textSummary(data, { indent: ' ', enableColors: true }) + report };
}

// ── Generación de eventos ──────────────────────────────────────────────────
const pick = (items) => items[Math.floor(Math.random() * items.length)];
const int = (min, max) => min + Math.floor(Math.random() * (max - min + 1));

function envelope(runId, source, event, withContext = true) {
  const e = {
    event_id: crypto.randomUUID(),
    user_id: `k6-${runId}-u${int(1, 50000)}`,
    occurred_at: new Date().toISOString(),
    source,
    event,
  };
  if (withContext) {
    e.context = {
      session_id: `s-${int(1, 1e6)}`,
      authenticated: Math.random() < 0.4,
      platform: pick(['web', 'ios', 'android', 'smart_tv']),
      device_type: pick(['desktop', 'mobile', 'tablet', 'tv']),
      app_version: pick(['3.2.0', '3.1.4', '2.9.0']),
      country: pick(['ES', 'MX', 'AR', 'CO', 'US']),
    };
  }
  return e;
}

function content() {
  return {
    id: `content-${int(1, 5000)}`,
    content_type: pick(['article', 'live_blog', 'video', 'live_stream', 'podcast', 'gallery']),
    section: pick(['deportes', 'economia', 'politica', 'cultura', 'tecnologia']),
    author_id: `author-${int(1, 300)}`,
    published_at: new Date(Date.now() - int(0, 72) * 3600 * 1000).toISOString(),
    paywalled: Math.random() < 0.2,
  };
}

// Distribución aproximada de un medio: dominan lecturas y latidos.
function validEvent(runId) {
  const roll = Math.random();
  if (roll < 0.35) {
    return envelope(runId, 'user_behavior', {
      type: 'content_viewed',
      content: content(),
      traffic_source: pick(['direct', 'search', 'social', 'newsletter', 'push_notification', 'internal', 'referral', 'paid']),
      referrer: pick([null, 'https://www.google.com/', 'https://t.co/x']),
      campaign: pick([null, 'otono-2026']),
    });
  }
  if (roll < 0.6) {
    return envelope(runId, 'user_behavior', {
      type: 'content_engaged',
      content: content(),
      engaged_secs: int(5, 600),
      scroll_depth: int(0, 100),
    });
  }
  if (roll < 0.8) {
    return envelope(runId, 'user_behavior', {
      type: 'playback_state_changed',
      content: content(),
      action: pick(['started', 'heartbeat', 'heartbeat', 'paused', 'resumed', 'seeked', 'completed', 'failed']),
      position_secs: int(0, 3600),
      duration_secs: pick([null, int(60, 7200)]),
      bitrate_kbps: int(800, 8000),
    });
  }
  if (roll < 0.86) {
    return envelope(runId, 'user_behavior', {
      type: 'recommendation_clicked',
      module: pick(['related', 'for_you', 'trending']),
      content: content(),
      position: int(0, 9),
    });
  }
  if (roll < 0.9) {
    return envelope(runId, 'user_behavior', {
      type: 'search_performed',
      query: pick(['elecciones', 'liga', 'ipc', 'tiempo', 'serie']),
      results_count: int(0, 500),
    });
  }
  if (roll < 0.94) {
    return envelope(runId, 'user_behavior', {
      type: 'ad_interacted',
      ad_id: `ad-${int(1, 200)}`,
      placement: pick(['pre_roll', 'mid_roll', 'post_roll', 'display', 'native']),
      action: pick(['impression', 'impression', 'click', 'skipped', 'completed']),
      content_id: `content-${int(1, 5000)}`,
    });
  }
  if (roll < 0.97) {
    return envelope(runId, 'user_behavior', { type: 'paywall_shown', content: content() });
  }
  return envelope(
    runId,
    'user_data',
    {
      type: 'subscription_changed',
      change: pick(['started', 'upgraded', 'downgraded', 'renewed', 'cancelled', 'expired']),
      plan: pick(['free', 'basic', 'premium']),
      previous_plan: pick([null, 'free', 'basic']),
      billing_period: pick(['monthly', 'yearly']),
      price: { amount_minor: pick([499, 999, 9900]), currency: 'EUR' },
      trial: Math.random() < 0.1,
    },
    false,
  );
}

// Errores típicos de cliente. Todos llevan `event_id`, así que un reintento no los duplica
// y cada uno acaba exactamente una vez en la DLQ.
function invalidEvent(runId) {
  const event = validEvent(runId);
  switch (int(0, 3)) {
    case 0: // SDK más nuevo que el procesador: tipo desconocido.
      event.event = { type: 'story_swiped' };
      event.source = 'user_behavior';
      return event;
    case 1: // Falta un campo obligatorio.
      delete event.user_id;
      return event;
    case 2: // Reloj del dispositivo desajustado: fuera de la ventana aceptada.
      event.occurred_at = new Date(Date.now() - 30 * 24 * 3600 * 1000).toISOString();
      return event;
    default: // Formato de fecha incorrecto.
      event.occurred_at = 'ayer por la tarde';
      return event;
  }
}

// ── Consultas al pipeline ──────────────────────────────────────────────────
function clickhouse(sql) {
  const res = http.post(`${CLICKHOUSE_URL}/?default_format=JSONEachRow`, sql, {
    headers: { 'X-ClickHouse-User': 'default', 'X-ClickHouse-Key': CLICKHOUSE_PASSWORD },
    tags: { name: 'clickhouse' },
  });
  if (res.status !== 200 || !res.body) {
    return null;
  }
  return JSON.parse(res.body.trim().split('\n')[0]);
}

// Eventos guardados de esta ejecución. Por defecto exacto (sin duplicados).
function storedCount(runId, aggregate = 'uniqExact(event_id)') {
  const row = clickhouse(
    `SELECT ${aggregate} AS v FROM events WHERE startsWith(user_id, 'k6-${runId}-')`,
  );
  return row === null ? -1 : Number(row.v);
}

function jetstream() {
  const res = http.get(`${NATS_MONITOR_URL}/jsz?streams=true&consumers=true`, { tags: { name: 'nats' } });
  return res.status === 200 ? res.json() : null;
}

function stream(name) {
  const jsz = jetstream();
  const details = jsz && jsz.account_details && jsz.account_details[0];
  const streams = (details && details.stream_detail) || [];
  return streams.find((s) => s.name === name) || null;
}

function consumerInfo() {
  const events = stream(NATS_STREAM);
  const consumers = (events && events.consumer_detail) || [];
  return consumers.find((c) => c.name === NATS_CONSUMER) || null;
}

function dlqMessages() {
  const dlq = stream(NATS_DLQ_STREAM);
  return dlq ? dlq.state.messages : 0;
}

// `addDurations('5m', '10m')` → '900s'. Solo s/m/h, suficiente para RAMP/HOLD.
function addDurations(...durations) {
  const seconds = (d) => {
    const match = /^(\d+)(s|m|h)$/.exec(d);
    if (!match) throw new Error(`duración no soportada: ${d}`);
    return Number(match[1]) * { s: 1, m: 60, h: 3600 }[match[2]];
  };
  return `${durations.reduce((total, d) => total + seconds(d), 0)}s`;
}
