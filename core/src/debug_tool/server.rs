//! HTTP server for debug MCP interface.

use std::{
  collections::VecDeque,
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::{SystemTime, UNIX_EPOCH},
};

use axum::{
  Json, Router,
  extract::{Path, Query, State},
  http::StatusCode,
  response::{
    Html, IntoResponse,
    sse::{Event, KeepAlive, Sse},
  },
  routing::{delete, get, post},
};
use futures::StreamExt;
use ribir_algo::Rc;
use ribir_painter::PixelImage;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_stream::wrappers::{BroadcastStream, IntervalStream};
use tower_http::cors::{Any, CorsLayer};

use super::{
  FRAME_TX, FramePacket, clear_overlays as clear_global_overlays,
  helpers::*,
  overlays::{get_overlays, remove_overlay},
  set_overlay_hex,
  types::*,
};
use crate::{
  context::AppCtx,
  prelude::WidgetId,
  widget_tree::WidgetTree,
  window::{Window, WindowId},
};

static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Debug)]
struct LogRingItem {
  ts_unix_ms: u64,
  line: Arc<str>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct FrameEntry {
  seq: u64,
  ts_unix_ms: u64,
  path: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct CaptureOptions {
  include_logs: bool,
  include_images: bool,
  pre_ms: u64,
  post_ms: u64,
}

#[derive(Debug)]
pub(crate) struct CaptureSession {
  id: String,
  dir: PathBuf,
  start_ts_unix_ms: u64,
  options: CaptureOptions,
  filter_at_start: Option<String>,
  frames: Vec<FrameEntry>,
}

#[derive(serde::Serialize)]
pub(crate) struct StatusCaptureInfo {
  capture_id: String,
  capture_dir: String,
  start_ts_unix_ms: u64,
  options: CaptureOptions,
  filter_at_start: Option<String>,
}

#[derive(serde::Serialize)]
pub(crate) struct StatusResponse {
  recording: bool,
  log_sink_connected: bool,
  filter_reload_installed: bool,
  filter: Option<String>,
  dropped_total: u64,
  ring_len: usize,
  capture_root: String,
  active_capture: Option<StatusCaptureInfo>,
}

#[derive(serde::Deserialize)]
pub(crate) struct CaptureStartRequest {
  pub(crate) include: Vec<String>,
  #[serde(default)]
  pub(crate) pre_ms: Option<u64>,
  #[serde(default)]
  pub(crate) post_ms: Option<u64>,
  #[serde(default)]
  pub(crate) output_dir: Option<String>,
}

#[derive(serde::Serialize)]
pub(crate) struct CaptureStartResponse {
  pub(crate) capture_id: String,
  pub(crate) capture_dir: String,
}

#[derive(serde::Deserialize, Default)]
pub(crate) struct CaptureStopRequest {
  #[serde(default)]
  pub(crate) capture_id: Option<String>,
}

#[derive(serde::Serialize)]
pub(crate) struct CaptureStopResponse {
  pub(crate) capture_id: String,
  pub(crate) capture_dir: String,
  pub(crate) manifest_path: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct CaptureOneShotRequest {
  pub(crate) include: Vec<String>,
  #[serde(default)]
  pub(crate) pre_ms: Option<u64>,
  #[serde(default)]
  pub(crate) post_ms: Option<u64>,
  /// Extra time (ms) to wait after we observe a frame update.
  #[serde(default)]
  pub(crate) settle_ms: Option<u64>,
  #[serde(default)]
  pub(crate) output_dir: Option<String>,
}

#[derive(Debug)]
pub struct LogRing {
  items: VecDeque<LogRingItem>,
  max_items: usize,
  max_age_ms: u64,
}

impl LogRing {
  fn new(max_items: usize, max_age_ms: u64) -> Self {
    Self { items: VecDeque::new(), max_items, max_age_ms }
  }

  fn push(&mut self, item: LogRingItem) {
    self.items.push_back(item);

    // Cap by size.
    while self.items.len() > self.max_items {
      self.items.pop_front();
    }

    // Cap by age.
    let newest_ts = self
      .items
      .back()
      .map(|i| i.ts_unix_ms)
      .unwrap_or(0);
    let min_ts = newest_ts.saturating_sub(self.max_age_ms);
    while self
      .items
      .front()
      .is_some_and(|i| i.ts_unix_ms < min_ts)
    {
      self.items.pop_front();
    }
  }

  pub fn query_lines(
    &self, since_ts: Option<u64>, until_ts: Option<u64>, limit: Option<usize>,
  ) -> Vec<Arc<str>> {
    let mut out = Vec::new();
    for item in self.items.iter() {
      if since_ts.is_some_and(|s| item.ts_unix_ms < s) {
        continue;
      }
      if until_ts.is_some_and(|u| item.ts_unix_ms > u) {
        continue;
      }
      out.push(item.line.clone());
      if limit.is_some_and(|l| out.len() >= l) {
        break;
      }
    }
    out
  }
}

#[derive(serde::Deserialize, Default)]
struct LogsQuery {
  since_ts: Option<u64>,
  until_ts: Option<u64>,
  limit: Option<usize>,
}

#[derive(serde::Deserialize)]
struct LogsFilterRequest {
  filter: String,
}

#[derive(serde::Serialize)]
struct LogsFilterResponse {
  ok: bool,
}

/// Shared state for the Axum handlers.
pub struct DebugServerState {
  pub command_tx: mpsc::Sender<DebugCommand>,
  pub recording: AtomicBool,
  pub last_frame_rx: watch::Receiver<Option<Arc<PixelImage>>>,
  pub last_frame_tx: watch::Sender<Option<Arc<PixelImage>>>,
  pub bound_addr: tokio::sync::RwLock<Option<std::net::SocketAddr>>,

  pub log_ring: tokio::sync::Mutex<LogRing>,
  pub log_broadcast: broadcast::Sender<Arc<str>>,

  pub capture_root: PathBuf,
  pub active_capture: tokio::sync::Mutex<Option<CaptureSession>>,
}

/// Start the debug HTTP server on a dynamically assigned port.
/// Start the debug HTTP server.
/// - Initializes debug logging.
pub fn start_debug_server() -> mpsc::Sender<DebugCommand> {
  crate::logging::init_debug_tracing("info");
  let bind_host = "127.0.0.1";
  let bind_start_port = 2333;
  let bind_port_range = 100;

  let (cmd_tx, mut cmd_rx) = mpsc::channel::<DebugCommand>(32);
  let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<FramePacket>();
  let (last_frame_tx, last_frame_rx) = watch::channel::<Option<Arc<PixelImage>>>(None);

  let (log_tx, mut log_rx) = mpsc::unbounded_channel::<crate::logging::LogLine>();
  crate::logging::install_debug_log_sender(log_tx);

  let (log_broadcast, _) = broadcast::channel::<Arc<str>>(1024);

  let capture_root = std::env::var("RIBIR_CAPTURE_DIR")
    .ok()
    .filter(|s| !s.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("captures"));

  // Initialize Global Frame Sender
  let _ = FRAME_TX.set(frame_tx);

  let state = Arc::new(DebugServerState {
    command_tx: cmd_tx.clone(),
    recording: AtomicBool::new(false),
    last_frame_rx,
    last_frame_tx,
    bound_addr: tokio::sync::RwLock::new(None),
    log_ring: tokio::sync::Mutex::new(LogRing::new(50_000, 60_000)),
    log_broadcast,

    capture_root,
    active_capture: tokio::sync::Mutex::new(None),
  });

  // Spawn Frame Processor Task (Background)
  let state_clone = state.clone();
  tokio::spawn(async move {
    while let Some(pkt) = frame_rx.recv().await {
      // 1. Update last frame (watch channel stores latest)
      let _ = state_clone
        .last_frame_tx
        .send_replace(Some(pkt.image.clone()));

      // 2. Save if recording
      if state_clone.recording.load(Ordering::Relaxed)
        && state_clone.active_capture.lock().await.is_none()
      {
        let filename = format!("frame_{}_{}.png", pkt.ts_unix_ms, pkt.seq);

        // Spawn blocking IO task to save image
        let img_clone = pkt.image.clone();
        tokio::task::spawn_blocking(move || {
          let mut data = Vec::new();
          if img_clone.write_as_png(&mut data).is_ok() {
            let _ = std::fs::write(filename, data);
          }
        });
      }

      // 3. Save into active capture session if enabled.
      // Note: we record the timestamp at receive-time on the server.
      let maybe_capture = {
        let mut guard = state_clone.active_capture.lock().await;
        if let Some(session) = guard.as_mut()
          && session.options.include_images
        {
          let ts_unix_ms = pkt.ts_unix_ms;
          let seq = pkt.seq;

          let rel_path = format!("frames/frame_{}_{}.png", ts_unix_ms, seq);
          let abs_path = session.dir.join(&rel_path);
          session
            .frames
            .push(FrameEntry { seq, ts_unix_ms, path: rel_path.clone() });
          Some((abs_path, pkt.image.clone()))
        } else {
          None
        }
      };

      if let Some((abs_path, img_clone)) = maybe_capture {
        tokio::task::spawn_blocking(move || {
          let _ = std::fs::create_dir_all(
            abs_path
              .parent()
              .unwrap_or_else(|| std::path::Path::new(".")),
          );
          let mut data = Vec::new();
          if img_clone.write_as_png(&mut data).is_ok() {
            let _ = std::fs::write(abs_path, data);
          }
        });
      }
    }
  });

  // Spawn Log Processor Task (Background)
  let log_state = state.clone();
  tokio::spawn(async move {
    while let Some(log_line) = log_rx.recv().await {
      let item = LogRingItem { ts_unix_ms: log_line.ts_unix_ms, line: log_line.line };
      {
        let mut ring = log_state.log_ring.lock().await;
        ring.push(item.clone());
      }
      let _ = log_state.log_broadcast.send(item.line);
    }
  });

  // Create HTTP router
  let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

  let app = Router::new()
    .route("/", get(ui_index))
    .route("/ui", get(ui_index))
    .route("/windows", get(get_windows))
    .route("/inspect/tree", get(inspect_tree))
    .route("/inspect/{id}", get(inspect_widget))
    .route("/overlay", post(add_overlay))
    .route("/overlay/{id}", delete(remove_overlay_by_id))
    .route("/overlays", get(get_overlays_handler))
    .route("/overlays", delete(clear_overlays))
    .route("/screenshot", get(capture_screenshot))
    .route("/status", get(get_status))
    .route("/recording", post(toggle_recording))
    .route("/logs", get(get_logs))
    .route("/logs/stream", get(stream_logs))
    .route("/logs/filter", post(set_logs_filter))
    .route("/capture/start", post(capture_start))
    .route("/capture/stop", post(capture_stop))
    .route("/capture/one_shot", post(capture_one_shot))
    // MCP protocol endpoints
    .route("/mcp/sse", get(mcp_sse_handler))
    .route("/mcp/message", post(mcp_message_handler))
    .layer(cors)
    .with_state(state.clone());

  // Spawn HTTP server on background task
  let state_clone2 = state.clone();
  tokio::spawn(async move {
    let mut listener = None;
    for offset in 0..bind_port_range {
      let port = bind_start_port + offset;
      let addr = format!("{}:{}", bind_host, port);
      match tokio::net::TcpListener::bind(&addr).await {
        Ok(found) => {
          listener = Some(found);
          break;
        }
        Err(err) => {
          tracing::warn!("Debug server bind failed on {}: {}", addr, err);
        }
      }
    }

    if listener.is_none() {
      match tokio::net::TcpListener::bind(format!("{}:0", bind_host)).await {
        Ok(found) => {
          listener = Some(found);
        }
        Err(e) => {
          tracing::error!("Failed to bind debug server on {}:0: {}", bind_host, e);
          return;
        }
      }
    }

    if let Some(listener) = listener {
      let local_addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(err) => {
          tracing::error!("Failed to read debug server address: {}", err);
          return;
        }
      };
      {
        let mut guard = state_clone2.bound_addr.write().await;
        *guard = Some(local_addr);
      }
      let port = local_addr.port();
      let url = format!("http://{}", local_addr);
      let ui_url = format!("{}/ui", url);
      tracing::info!("Debug server listening on {} (open /ui)", url);
      println!("Debug server listening on {} (open /ui)", url);
      eprintln!("RIBIR_DEBUG_URL={}", url);
      eprintln!("RIBIR_DEBUG_UI={}", ui_url);

      // Register the port for discovery by MCP clients
      let registry_file = super::port_registry::register_port(port).ok();

      let result = axum::serve(listener, app).await;

      // Unregister on shutdown
      if let Some(file) = registry_file {
        super::port_registry::unregister_port(&file);
      }

      result.ok();
    }
  });

  // Spawn command handler on UI thread
  AppCtx::spawn_local(async move {
    while let Some(cmd) = cmd_rx.recv().await {
      handle_command(cmd).await;
    }
  });

  cmd_tx
}

/// GET / (and /ui)
///
/// A tiny built-in UI to make the debug server convenient to use.
/// It attempts to load `ui.html` from source for dev iteration, falling back to
/// `include_str!`.
async fn ui_index() -> impl IntoResponse {
  let content = std::fs::read_to_string(DEBUG_SERVER_UI_PATH)
    .unwrap_or_else(|_| DEBUG_SERVER_UI_HTML.to_string());
  Html(content)
}

const DEBUG_SERVER_UI_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/debug_tool/ui.html");
const DEBUG_SERVER_UI_HTML: &str = include_str!("ui.html");

/// GET /windows
async fn get_windows(
  State(state): State<Arc<DebugServerState>>,
) -> Result<Json<Vec<WindowInfo>>, StatusCode> {
  use crate::debug_tool::service::*;
  match get_windows_svc(&state).await {
    Ok(windows) => Ok(Json(windows)),
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}

/// GET /status
async fn get_status(State(state): State<Arc<DebugServerState>>) -> Json<StatusResponse> {
  Json(build_status_response(&state).await)
}

pub(crate) async fn build_status_response(state: &DebugServerState) -> StatusResponse {
  let ring_len = { state.log_ring.lock().await.items.len() };

  let active_capture = {
    let guard = state.active_capture.lock().await;
    guard.as_ref().map(|c| StatusCaptureInfo {
      capture_id: c.id.clone(),
      capture_dir: c.dir.to_string_lossy().to_string(),
      start_ts_unix_ms: c.start_ts_unix_ms,
      options: c.options.clone(),
      filter_at_start: c.filter_at_start.clone(),
    })
  };

  StatusResponse {
    recording: state.recording.load(Ordering::Relaxed),
    log_sink_connected: crate::logging::debug_log_sender_installed(),
    filter_reload_installed: crate::logging::current_filter_reload_installed(),
    filter: crate::logging::current_filter_string(),
    dropped_total: crate::logging::dropped_logs_total(),
    ring_len,
    capture_root: state.capture_root.to_string_lossy().to_string(),
    active_capture,
  }
}

/// POST /logs/filter
/// Body: {"filter": "info,ribir_core=debug"}
async fn set_logs_filter(
  Json(payload): Json<LogsFilterRequest>,
) -> Result<Json<LogsFilterResponse>, (StatusCode, String)> {
  crate::logging::update_filter(&payload.filter).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
  Ok(Json(LogsFilterResponse { ok: true }))
}

#[derive(serde::Deserialize, Default)]
struct LayoutQuery {
  #[serde(default)]
  options: Option<String>,
  #[serde(default)]
  window_id: Option<WindowId>,
}

/// POST /capture/start
async fn capture_start(
  State(state): State<Arc<DebugServerState>>, Json(payload): Json<CaptureStartRequest>,
) -> Result<Json<CaptureStartResponse>, StatusCode> {
  capture_start_inner(
    state,
    payload.include,
    payload.pre_ms.unwrap_or(2_000),
    payload.post_ms.unwrap_or(1_000),
    payload.output_dir,
  )
  .await
}

pub(crate) async fn capture_start_inner(
  state: Arc<DebugServerState>, include: Vec<String>, pre_ms: u64, post_ms: u64,
  output_dir: Option<String>,
) -> Result<Json<CaptureStartResponse>, StatusCode> {
  let include_logs = include.iter().any(|s| s == "logs");
  let include_images = include.iter().any(|s| s == "images");

  let options = CaptureOptions { include_logs, include_images, pre_ms, post_ms };

  let mut guard = state.active_capture.lock().await;
  if guard.is_some() {
    return Err(StatusCode::CONFLICT);
  }

  let start_ts_unix_ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64;

  let seq = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  let capture_id = format!("cap_{}_{}", start_ts_unix_ms, seq);

  let root = output_dir
    .as_ref()
    .map(PathBuf::from)
    .unwrap_or_else(|| state.capture_root.clone());
  let capture_dir = absolutize_path(root.join(&capture_id));

  // Ensure dirs exist.
  let capture_dir_clone = capture_dir.clone();
  tokio::task::spawn_blocking(move || {
    let _ = std::fs::create_dir_all(capture_dir_clone.join("frames"));
  })
  .await
  .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

  *guard = Some(CaptureSession {
    id: capture_id.clone(),
    dir: capture_dir.clone(),
    start_ts_unix_ms,
    options,
    filter_at_start: crate::logging::current_filter_string(),
    frames: Vec::new(),
  });

  Ok(Json(CaptureStartResponse {
    capture_id,
    capture_dir: capture_dir.to_string_lossy().to_string(),
  }))
}

/// POST /capture/stop
async fn capture_stop(
  State(state): State<Arc<DebugServerState>>, Json(payload): Json<CaptureStopRequest>,
) -> Result<Json<CaptureStopResponse>, StatusCode> {
  capture_stop_inner(state, payload).await
}

pub(crate) async fn capture_stop_inner(
  state: Arc<DebugServerState>, payload: CaptureStopRequest,
) -> Result<Json<CaptureStopResponse>, StatusCode> {
  let end_ts_unix_ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64;

  let session = {
    let mut guard = state.active_capture.lock().await;
    let Some(session) = guard.as_ref() else {
      return Err(StatusCode::NOT_FOUND);
    };
    if let Some(req_id) = payload.capture_id.as_ref()
      && req_id != &session.id
    {
      return Err(StatusCode::NOT_FOUND);
    }
    guard.take().unwrap()
  };

  let capture_id = session.id.clone();
  let capture_id_for_manifest = capture_id.clone();
  let capture_dir = session.dir.clone();
  let manifest_path = capture_dir.join("manifest.json");
  let start_ts_unix_ms = session.start_ts_unix_ms;

  let capture_dir_str = absolutize_path(&capture_dir)
    .to_string_lossy()
    .to_string();
  let manifest_path_str = absolutize_path(&manifest_path)
    .to_string_lossy()
    .to_string();

  let logs_since = session
    .start_ts_unix_ms
    .saturating_sub(session.options.pre_ms);
  let logs_until = end_ts_unix_ms.saturating_add(session.options.post_ms);
  let log_lines: Vec<Arc<str>> = if session.options.include_logs {
    let ring = state.log_ring.lock().await;
    ring.query_lines(Some(logs_since), Some(logs_until), None)
  } else {
    Vec::new()
  };

  let frames = session.frames.clone();
  let options = session.options.clone();
  let filter_at_start = session.filter_at_start.clone();
  let dropped = crate::logging::dropped_logs_total();

  tokio::task::spawn_blocking(move || {
    let _ = std::fs::create_dir_all(&capture_dir);

    if options.include_logs {
      let logs_path = capture_dir.join("logs.jsonl");
      if let Ok(mut f) = std::fs::File::create(&logs_path) {
        for line in &log_lines {
          let _ = std::io::Write::write_all(&mut f, line.as_bytes());
          let _ = std::io::Write::write_all(&mut f, b"\n");
        }
      }
    }

    let manifest = serde_json::json!({
      "schema_version": 1,
      "capture_id": capture_id_for_manifest,
      "start_ts_unix_ms": start_ts_unix_ms,
      "end_ts_unix_ms": end_ts_unix_ms,
      "options": {
        "include_logs": options.include_logs,
        "include_images": options.include_images,
        "pre_ms": options.pre_ms,
        "post_ms": options.post_ms,
        "filter_at_start": filter_at_start,
      },
      "logs": options.include_logs.then(|| {
        serde_json::json!({
          "path": "logs.jsonl",
          "count": log_lines.len(),
          "dropped_total": dropped,
          "since_ts_unix_ms": logs_since,
          "until_ts_unix_ms": logs_until,
        })
      }),
      "frames": options.include_images.then_some(frames),
    });

    let _ =
      std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest).unwrap_or_default());
  })
  .await
  .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

  Ok(Json(CaptureStopResponse {
    capture_id,
    capture_dir: capture_dir_str,
    manifest_path: manifest_path_str,
  }))
}

/// POST /capture/one_shot
///
/// One-click capture: start -> request redraw (if images enabled) -> wait ->
/// stop.
async fn capture_one_shot(
  State(state): State<Arc<DebugServerState>>, Json(payload): Json<CaptureOneShotRequest>,
) -> Result<Json<CaptureStopResponse>, StatusCode> {
  capture_one_shot_inner(state, payload).await
}

pub(crate) async fn capture_one_shot_inner(
  state: Arc<DebugServerState>, payload: CaptureOneShotRequest,
) -> Result<Json<CaptureStopResponse>, StatusCode> {
  let settle_ms = payload.settle_ms.unwrap_or(150);
  let include_images = payload.include.iter().any(|s| s == "images");

  let _start = capture_start_inner(
    state.clone(),
    payload.include,
    payload.pre_ms.unwrap_or(2_000),
    payload.post_ms.unwrap_or(1_000),
    payload.output_dir,
  )
  .await?;

  if include_images {
    let initial_frame_count = {
      let guard = state.active_capture.lock().await;
      guard
        .as_ref()
        .map(|s| s.frames.len())
        .unwrap_or(0)
    };

    // Ask the UI thread to redraw once, then wait for the next frame update.
    let mut rx = state.last_frame_rx.clone();
    let _ = rx.borrow_and_update();
    let _ = state
      .command_tx
      .send(DebugCommand::RequestRedraw { window_id: None })
      .await;

    // Wait until we observe at least one new frame update (best-effort).
    let waited_new_frame = tokio::time::timeout(std::time::Duration::from_millis(1200), async {
      loop {
        if rx.changed().await.is_err() {
          return false;
        }
        let current = {
          let guard = state.active_capture.lock().await;
          guard
            .as_ref()
            .map(|s| s.frames.len())
            .unwrap_or(0)
        };
        if current > initial_frame_count {
          return true;
        }
      }
    })
    .await
    .unwrap_or(false);

    // Optional extra settle time to capture any overlays/layout changes.
    tokio::time::sleep(std::time::Duration::from_millis(settle_ms)).await;
    // Allow spawned PNG writes to flush before stopping capture.
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    if !waited_new_frame {
      let _ = capture_stop_inner(state.clone(), CaptureStopRequest { capture_id: None }).await;
      return Err(StatusCode::REQUEST_TIMEOUT);
    }
  }

  capture_stop_inner(state, CaptureStopRequest { capture_id: None }).await
}

/// GET /logs
///
/// Returns NDJSON lines (one JSON event per line), consistent with capture
/// output.
async fn get_logs(
  State(state): State<Arc<DebugServerState>>, Query(query): Query<LogsQuery>,
) -> impl IntoResponse {
  let lines = {
    let ring = state.log_ring.lock().await;
    ring.query_lines(query.since_ts, query.until_ts, query.limit)
  };

  let mut out = String::new();
  for line in lines {
    out.push_str(line.as_ref());
    out.push('\n');
  }

  let dropped = crate::logging::dropped_logs_total();

  let mut headers = axum::http::HeaderMap::new();
  headers.insert(
    axum::http::header::CONTENT_TYPE,
    axum::http::HeaderValue::from_static("application/x-ndjson; charset=utf-8"),
  );
  headers.insert(
    axum::http::HeaderName::from_static("x-ribir-log-dropped"),
    axum::http::HeaderValue::from_str(&dropped.to_string())
      .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
  );

  (StatusCode::OK, headers, out)
}

/// GET /logs/stream
///
/// Server-Sent Events stream of NDJSON log events.
async fn stream_logs(
  State(state): State<Arc<DebugServerState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
  let rx = state.log_broadcast.subscribe();
  let logs_stream = BroadcastStream::new(rx).map(|msg| {
    let event = match msg {
      Ok(line) => Event::default().event("log").data(line.as_ref()),
      Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => Event::default()
        .event("lagged")
        .data(n.to_string()),
    };
    Ok(event)
  });

  // Periodic stats updates so clients don't need to poll /status.
  let stats_state = state.clone();
  let ticker = tokio::time::interval(std::time::Duration::from_secs(2));
  let stats_stream = IntervalStream::new(ticker).then(move |_| {
    let stats_state = stats_state.clone();
    async move {
      let ring_len = { stats_state.log_ring.lock().await.items.len() };
      let payload = serde_json::json!({
        "filter": crate::logging::current_filter_string(),
        "dropped_total": crate::logging::dropped_logs_total(),
        "ring_len": ring_len,
        "recording": stats_state.recording.load(Ordering::Relaxed),
      });
      Ok(
        Event::default()
          .event("stats")
          .data(payload.to_string()),
      )
    }
  });

  let stream = futures::stream::select(logs_stream, stats_stream);

  Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

/// Handle a debug command on the UI thread.
fn resolve_widget_id(id_str: &str, tree: &WidgetTree) -> Option<WidgetId> {
  if let Ok(wid) = serde_json::from_str::<WidgetId>(id_str) {
    return Some(wid);
  }

  if let Some((index, stamp)) = id_str.split_once(':')
    && let (Ok(index), Ok(stamp)) = (index.parse::<u64>(), stamp.parse::<u64>())
  {
    return find_widget_id_by_parts(tree, index, Some(stamp));
  }

  if let Ok(idx) = id_str.parse::<u64>() {
    return find_widget_id_by_parts(tree, idx, None);
  }

  None
}

fn find_widget_id_by_parts(tree: &WidgetTree, index1: u64, stamp: Option<u64>) -> Option<WidgetId> {
  let root = tree.root();
  let mut stack = vec![root];
  while let Some(node) = stack.pop() {
    if let Ok(val) = serde_json::to_value(node) {
      let node_index = val.get("index1").and_then(|v| v.as_u64());
      let node_stamp = val.get("stamp").and_then(|v| v.as_u64());
      let stamp_match = match stamp {
        Some(expected) => node_stamp == Some(expected),
        None => true,
      };
      if node_index == Some(index1) && stamp_match {
        return Some(node);
      }
    }
    stack.extend(node.children(tree));
  }
  None
}

fn absolutize_path(path: impl Into<PathBuf>) -> PathBuf {
  let path = path.into();
  if path.is_absolute() {
    return path;
  }
  std::env::current_dir()
    .map(|cwd| cwd.join(&path))
    .unwrap_or(path)
}

fn resolve_target_window(requested_id: Option<WindowId>) -> Option<Rc<Window>> {
  if let Some(id) = requested_id {
    AppCtx::get_window(id)
  } else {
    AppCtx::windows()
      .borrow()
      .values()
      .next()
      .cloned()
  }
}

async fn handle_command(cmd: DebugCommand) {
  match cmd {
    DebugCommand::GetWindows { reply } => {
      let windows = AppCtx::windows()
        .borrow()
        .values()
        .map(|w| {
          let shell = w.shell_wnd().borrow();
          let size = shell.inner_size();
          WindowInfo {
            id: w.id(),
            title: format!("Window {:?}", w.id()),
            width: size.width,
            height: size.height,
          }
        })
        .collect();
      let _ = reply.send(windows);
    }
    DebugCommand::InspectWidgetTree { window_id, options, reply } => {
      if let Some(wnd) = resolve_target_window(window_id) {
        let tree = wnd.tree();
        let root = tree.root();
        let node = build_layout_tree_json(root, tree, options);
        let _ = reply.send(node);
      }
    }

    DebugCommand::InspectWidget { window_id, id, options, reply } => {
      if let Some(wnd) = resolve_target_window(window_id) {
        let tree = wnd.tree();
        let widget_id = resolve_widget_id(&id, tree);
        let info =
          if let Some(wid) = widget_id { build_layout_info_json(wid, tree, options) } else { None };
        let _ = reply.send(info);
      } else {
        let _ = reply.send(None);
      }
    }

    DebugCommand::AddOverlay { window_id, id, color, reply } => {
      if let Some(wnd) = resolve_target_window(window_id) {
        let tree = wnd.tree();
        let widget_id = resolve_widget_id(&id, tree);
        let success = if let Some(wid) = widget_id {
          set_overlay_hex(wnd.id(), wid, &color).is_some()
        } else {
          false
        };

        let _ = reply.send(success);
      } else {
        let _ = reply.send(false);
      }
    }

    DebugCommand::RemoveOverlay { window_id, id, reply } => {
      if let Some(wnd) = resolve_target_window(window_id) {
        let tree = wnd.tree();
        let widget_id = resolve_widget_id(&id, tree);

        let success =
          if let Some(wid) = widget_id { remove_overlay(wnd.id(), wid).is_some() } else { false };

        let _ = reply.send(success);
      } else {
        let _ = reply.send(false);
      }
    }

    DebugCommand::ClearOverlays { window_id } => {
      if let Some(wnd) = resolve_target_window(window_id) {
        clear_global_overlays(Some(wnd.id()));
      } else if window_id.is_none() {
        clear_global_overlays(None);
      }
    }

    DebugCommand::RequestRedraw { window_id } => {
      if let Some(wnd) = resolve_target_window(window_id) {
        wnd.shell_wnd().borrow().request_draw(true);
      }
    }
  }
}

/// GET /inspect/tree - Returns the full widget tree.
async fn inspect_tree(
  State(state): State<Arc<DebugServerState>>, Query(q): Query<LayoutQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
  use crate::debug_tool::service::*;
  let options = parse_options(q.options.as_deref());
  match inspect_tree_svc(&state, q.window_id, options).await {
    Ok(tree) => Ok(Json(tree)),
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}

/// GET /inspect/{id} - Returns detailed layout info for a specific widget.
async fn inspect_widget(
  State(state): State<Arc<DebugServerState>>, Path(id): Path<String>, Query(q): Query<LayoutQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
  use crate::debug_tool::service::*;
  let options = parse_options(q.options.as_deref());
  match inspect_widget_svc(&state, q.window_id, id, options).await {
    Ok(info) => Ok(Json(info)),
    Err(ServiceError::NotFound) => Err(StatusCode::NOT_FOUND),
    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
  }
}

#[derive(serde::Deserialize, Default)]
struct OverlayQuery {
  #[serde(default)]
  window_id: Option<WindowId>,
}

/// POST /overlay - Add a debug overlay.
async fn add_overlay(
  State(state): State<Arc<DebugServerState>>, Json(payload): Json<OverlayRequest>,
) -> StatusCode {
  use crate::debug_tool::service::*;
  match add_overlay_svc(&state, payload.window_id, payload.id, payload.color).await {
    Ok(()) => StatusCode::OK,
    Err(ServiceError::NotFound) => StatusCode::BAD_REQUEST,
    Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
  }
}

/// GET /overlays - List all overlays.
async fn get_overlays_handler(Query(q): Query<OverlayQuery>) -> Json<Vec<(WidgetId, String)>> {
  if let Some(wnd) = resolve_target_window(q.window_id) {
    Json(get_overlays(wnd.id()))
  } else {
    Json(vec![])
  }
}

/// DELETE /overlay/{id} - Remove a specific overlay.
async fn remove_overlay_by_id(
  State(state): State<Arc<DebugServerState>>, Path(id): Path<String>, Query(q): Query<OverlayQuery>,
) -> StatusCode {
  use crate::debug_tool::service::*;
  match remove_overlay_svc(&state, q.window_id, id).await {
    Ok(()) => StatusCode::OK,
    Err(ServiceError::NotFound) => StatusCode::NOT_FOUND,
    Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
  }
}

/// DELETE /overlays - Clear all debug overlays.
async fn clear_overlays(
  State(state): State<Arc<DebugServerState>>, Query(q): Query<OverlayQuery>,
) -> StatusCode {
  use crate::debug_tool::service::*;
  match clear_overlays_svc(&state, q.window_id).await {
    Ok(()) => StatusCode::OK,
    Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
  }
}

/// POST /recording - Toggle recording.
/// Body: `{"enable": true}`
#[derive(serde::Deserialize)]
struct RecordingRequest {
  enable: bool,
}

async fn toggle_recording(
  State(state): State<Arc<DebugServerState>>, Json(payload): Json<RecordingRequest>,
) -> StatusCode {
  state
    .recording
    .store(payload.enable, Ordering::Relaxed);
  StatusCode::OK
}

/// GET /screenshot
async fn capture_screenshot(State(state): State<Arc<DebugServerState>>) -> impl IntoResponse {
  use crate::debug_tool::service::*;
  match capture_screenshot_svc(&state).await {
    Ok(img) => encode_png_response(&img),
    Err(ServiceError::Timeout) => (StatusCode::REQUEST_TIMEOUT, Vec::new()).into_response(),
    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Vec::new()).into_response(),
  }
}

fn encode_png_response(img: &PixelImage) -> axum::response::Response {
  let mut png_data = Vec::new();
  if img.write_as_png(&mut png_data).is_ok() {
    (StatusCode::OK, [("content-type", "image/png")], png_data).into_response()
  } else {
    (StatusCode::INTERNAL_SERVER_ERROR, Vec::new()).into_response()
  }
}

// === MCP Protocol Handlers ===

/// GET /mcp/sse
/// SSE endpoint for MCP (Model Context Protocol)
///
/// Per MCP spec, this endpoint sends an "endpoint" event with the URL
/// where clients should POST their JSON-RPC requests.
async fn mcp_sse_handler(
  State(state): State<Arc<DebugServerState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
  tracing::info!("MCP: SSE connection established");

  // Build the message endpoint URL
  let endpoint_url = {
    let guard = state.bound_addr.read().await;
    let addr = guard
      .as_ref()
      .map(|a| a.to_string())
      .unwrap_or_else(|| "127.0.0.1:2333".to_string());
    format!("http://{}/mcp/message", addr)
  };

  // Send the endpoint event as required by MCP spec
  let endpoint_event = Event::default()
    .event("endpoint")
    .data(endpoint_url);

  // Create a stream that first sends the endpoint event, then keeps the
  // connection alive
  let stream = futures::stream::once(async move { Ok(endpoint_event) });

  // Keep the SSE connection alive - the keep_alive will send comments
  // periodically
  Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

/// POST /mcp/message
/// Handles JSON-RPC requests from MCP clients
async fn mcp_message_handler(
  State(state): State<Arc<DebugServerState>>, Json(payload): Json<super::mcp::JsonRpcRequest>,
) -> Json<super::mcp::JsonRpcResponse> {
  tracing::info!("MCP: message method={}", payload.method);

  let response = super::mcp::handle_mcp_request(payload, state).await;
  Json(response)
}
