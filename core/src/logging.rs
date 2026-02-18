//! Debug-focused tracing integration.
//!
//! This module is intentionally gated behind the `debug` feature.
//!
//! It provides:
//! - a best-effort global `tracing` subscriber initialization
//! - a `log` -> `tracing` bridge
//! - a lightweight layer that forwards structured JSON events to the debug
//!   server server

use std::{
  fmt,
  sync::{
    OnceLock, RwLock,
    atomic::{AtomicU64, Ordering},
  },
  time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Map, Value};
use tokio::sync::mpsc;
use tracing::{Event, Subscriber};
use tracing_subscriber::{EnvFilter, Layer, layer::Context, prelude::*, registry::LookupSpan};

type ReloadHandle = tracing_subscriber::reload::Handle<EnvFilter, tracing_subscriber::Registry>;

/// A single log record encoded as one-line JSON (NDJSON).
#[derive(Clone, Debug)]
pub struct LogLine {
  pub ts_unix_ms: u64,
  pub line: std::sync::Arc<str>,
}

static DEBUG_LOG_TX: OnceLock<mpsc::UnboundedSender<LogLine>> = OnceLock::new();
static DROPPED_LOGS: AtomicU64 = AtomicU64::new(0);
static FILTER_RELOAD: OnceLock<ReloadHandle> = OnceLock::new();
static CURRENT_FILTER: OnceLock<RwLock<String>> = OnceLock::new();

/// Install the debug server log sink.
///
/// Safe to call multiple times; only the first call wins.
pub fn install_debug_log_sender(tx: mpsc::UnboundedSender<LogLine>) {
  let _ = DEBUG_LOG_TX.set(tx);
}

pub fn debug_log_sender_installed() -> bool { DEBUG_LOG_TX.get().is_some() }

pub fn dropped_logs_total() -> u64 { DROPPED_LOGS.load(Ordering::Relaxed) }

pub fn current_filter_reload_installed() -> bool { FILTER_RELOAD.get().is_some() }

pub fn current_filter_string() -> Option<String> {
  CURRENT_FILTER
    .get()
    .and_then(|l| l.read().ok().map(|g| g.clone()))
}

fn set_current_filter_string(filter: &str) {
  let lock = CURRENT_FILTER.get_or_init(|| RwLock::new(String::new()));
  if let Ok(mut g) = lock.write() {
    *g = filter.to_owned();
  }
}

pub fn update_filter(filter: &str) -> Result<(), String> {
  let handle = FILTER_RELOAD
    .get()
    .ok_or_else(|| "filter reload not installed".to_owned())?;
  let new_filter = EnvFilter::try_new(filter).map_err(|e| format!("invalid filter: {e}"))?;
  handle
    .modify(|f| {
      *f = new_filter;
    })
    .map_err(|e| format!("failed to update filter: {e}"))?;

  set_current_filter_string(filter);
  Ok(())
}

fn now_unix_ms() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64
}

/// Initialize a global tracing subscriber for debug scenarios.
///
/// - Uses `try_init` so it will NOT override an existing global subscriber.
/// - Installs a `log` -> `tracing` bridge (best-effort).
/// - Adds [`DebugMcpLogLayer`] so the debug server can receive logs.
///
/// `default_filter` is used when neither `RIBIR_LOG` nor `RUST_LOG` is set.
pub fn init_debug_tracing(default_filter: &str) {
  let _ = tracing_log::LogTracer::init();

  // Prefer a dedicated env var, but fall back to standard RUST_LOG.
  let filter = std::env::var("RIBIR_LOG")
    .ok()
    .or_else(|| std::env::var("RUST_LOG").ok())
    .unwrap_or_else(|| default_filter.to_owned());

  set_current_filter_string(&filter);

  let env_filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new(default_filter));

  let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(env_filter);
  let _ = FILTER_RELOAD.set(reload_handle);

  let subscriber = tracing_subscriber::registry()
    .with(filter_layer)
    .with(DebugMcpLogLayer);

  let _ = tracing::subscriber::set_global_default(subscriber);
}

struct JsonVisitor {
  fields: Map<String, Value>,
}

impl JsonVisitor {
  fn new() -> Self { Self { fields: Map::new() } }
}

impl tracing::field::Visit for JsonVisitor {
  fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value));
  }

  fn record_error(
    &mut self, field: &tracing::field::Field, value: &(dyn std::error::Error + 'static),
  ) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(value.to_string()));
  }

  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
    self
      .fields
      .insert(field.name().to_owned(), Value::from(format!("{value:?}")));
  }
}

/// A lightweight `tracing` layer that forwards events as one-line JSON into the
/// debug server (when installed).
struct DebugMcpLogLayer;

impl<S> Layer<S> for DebugMcpLogLayer
where
  S: Subscriber + for<'a> LookupSpan<'a>,
{
  fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
    let Some(tx) = DEBUG_LOG_TX.get() else {
      return;
    };

    let ts_unix_ms = now_unix_ms();

    let meta = event.metadata();
    let mut visitor = JsonVisitor::new();
    event.record(&mut visitor);

    // Flatten span context into an array for easier consumption.
    let spans: Vec<Value> = if let Some(scope) = ctx.event_scope(event) {
      scope
        .from_root()
        .map(|span| {
          let mut span_obj = Map::new();
          span_obj.insert("name".to_owned(), Value::from(span.name()));
          Value::Object(span_obj)
        })
        .collect()
    } else {
      Vec::new()
    };

    let mut obj = Map::new();
    obj.insert("ts_unix_ms".to_owned(), Value::from(ts_unix_ms));
    obj.insert("level".to_owned(), Value::from(meta.level().as_str()));
    obj.insert("target".to_owned(), Value::from(meta.target()));
    obj.insert("fields".to_owned(), Value::Object(visitor.fields));
    obj.insert("spans".to_owned(), Value::Array(spans));

    if let Some(file) = meta.file() {
      obj.insert("file".to_owned(), Value::from(file));
    }
    if let Some(line) = meta.line() {
      obj.insert("line".to_owned(), Value::from(line as u64));
    }

    let line = match serde_json::to_string(&Value::Object(obj)) {
      Ok(s) => s,
      Err(_) => return,
    };

    let log_line = LogLine { ts_unix_ms, line: std::sync::Arc::from(line) };
    if tx.send(log_line).is_err() {
      DROPPED_LOGS.fetch_add(1, Ordering::Relaxed);
    }
  }
}

// NOTE: Span fields are intentionally omitted in the first iteration.
