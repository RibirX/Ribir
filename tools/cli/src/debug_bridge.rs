//! WASM Debug Bridge Server
//!
//! This module provides an HTTP server that acts as a bridge between
//! debug tools (like the web UI) and WASM applications.
//!
//! ## Architecture
//!
//! ```text
//! Debug Tool (Web UI) <--HTTP--> Bridge Server <--WebSocket--> WASM App
//! ```
//!
//! The Bridge Server:
//! 1. Serves the debug UI HTML page
//! 2. Accepts HTTP requests from debug tools
//! 3. Forwards requests to connected WASM applications via WebSocket
//! 4. Returns responses from WASM back to the debug tools

use std::{
  collections::HashMap,
  path::PathBuf,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use anyhow::Result;
use axum::{
  Json, Router,
  extract::{
    Path, Query, State,
    ws::{Message, WebSocket, WebSocketUpgrade},
  },
  http::StatusCode,
  response::{
    Html, IntoResponse, Response,
    sse::{Event, KeepAlive, Sse},
  },
  routing::{delete, get, post},
};
use clap::{Args, CommandFactory, FromArgMatches, Parser};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_stream::wrappers::IntervalStream;
use tower_http::cors::{Any, CorsLayer};

use crate::CliCommand;

pub fn debug_server() -> Box<dyn CliCommand> { Box::new(DebugServerCmd) }

const DEFAULT_FORWARD_TIMEOUT_MS: u64 = 15_000;
const MACRO_START_TIMEOUT_GRACE_MS: u64 = 5_000;

pub async fn run_server(args: DebugServerArgs, state: Arc<BridgeState>) -> Result<()> {
  let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

  let app = Router::new()
    // UI and basic routes
    .route("/", get(get_ui))
    .route("/ui", get(get_ui))
    .route("/status", get(get_status))
    .route("/windows", get(get_windows))
    // Layout inspection
    .route("/inspect/tree", get(inspect_tree))
    .route("/inspect/{id}", get(inspect_widget))
    // Overlay management
    .route("/overlay", post(add_overlay))
    .route("/overlay/{id}", delete(remove_overlay))
    .route("/overlays", get(get_overlays))
    .route("/overlays", delete(clear_overlays))
    // Screenshot
    .route("/screenshot", get(capture_screenshot))
    // Recording
    .route("/recording", post(toggle_recording))
    // Logs
    .route("/logs", get(get_logs))
    .route("/logs/stream", get(stream_logs))
    .route("/logs/filter", post(set_logs_filter))
    // Events
    .route("/events/inject", post(inject_events))
    // Event macro recording
    .route("/events/macro/start", post(start_event_macro_recording))
    .route("/events/macro/stop", post(stop_event_macro_recording))
    // Capture transport
    .route("/capture/start", post(capture_start))
    .route("/capture/stop", post(capture_stop))
    .route("/capture/one_shot", post(capture_one_shot))
    // WebSocket for WASM connection
    .route("/ws", get(ws_handler))
    .layer(cors)
    .with_state(state.clone());

  let mut listener = None;
  for offset in 0..100 {
    let port = args.port.saturating_add(offset);
    let addr = format!("{}:{}", args.host, port);
    match tokio::net::TcpListener::bind(&addr).await {
      Ok(found) => {
        listener = Some(found);
        break;
      }
      Err(err) => {
        tracing::warn!("Debug bridge bind failed on {}: {}", addr, err);
      }
    }
  }

  if listener.is_none() {
    listener = Some(tokio::net::TcpListener::bind(format!("{}:0", args.host)).await?);
  }

  let listener = listener.expect("bridge listener should exist");
  let addr = listener.local_addr()?;
  let http_url = format!("http://{}", addr);
  eprintln!("RIBIR_DEBUG_URL={}", http_url);

  axum::serve(listener, app).await?;
  Ok(())
}

#[derive(Parser, Debug)]
#[command(name = "debug-server")]
struct DebugServerCli {
  /// Host to bind.
  #[arg(long, default_value = "127.0.0.1")]
  pub host: String,

  /// Port to bind.
  #[arg(long, default_value_t = 2333)]
  pub port: u16,
}

#[derive(Args, Debug, Clone)]
pub struct DebugServerArgs {
  /// Host to bind.
  #[arg(long, default_value = "127.0.0.1")]
  pub host: String,

  /// Port to bind.
  #[arg(long, default_value_t = 2333)]
  pub port: u16,
}

struct DebugServerCmd;

impl CliCommand for DebugServerCmd {
  fn name(&self) -> &str { "debug-server" }

  fn command(&self) -> clap::Command { DebugServerCli::command() }

  fn exec(&self, args: &clap::ArgMatches) -> Result<()> {
    let cli_args = DebugServerCli::from_arg_matches(args)?;
    let args = DebugServerArgs { host: cli_args.host, port: cli_args.port };
    tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()?
      .block_on(run_server(args, Arc::new(BridgeState::default())))
  }
}

// === Bridge State ===

struct ActiveSession {
  token: u64,
  sender: mpsc::UnboundedSender<Message>,
  page_url: Option<String>,
}

struct PendingRequest {
  #[allow(dead_code)]
  token: u64,
  reply: oneshot::Sender<Value>,
}

#[derive(Clone, Debug)]
struct BridgeCaptureSession {
  capture_id: String,
  capture_dir: PathBuf,
}

#[derive(Default)]
pub struct BridgeState {
  active: Mutex<Option<ActiveSession>>,
  pending: Mutex<HashMap<u64, PendingRequest>>,
  next_request_id: AtomicU64,
  next_session_token: AtomicU64,
  active_capture: Mutex<Option<BridgeCaptureSession>>,
}

impl BridgeState {}

// === Protocol Messages ===

/// Request from Bridge to WASM client
#[derive(Serialize)]
struct BridgeRequest {
  request_id: u64,
  path: String,
  method: String,
  body: Option<Value>,
}

/// Response from WASM client to Bridge
#[derive(Deserialize, Serialize)]
struct BridgeResponse {
  request_id: u64,
  status: u16,
  body: Value,
}

/// Hello message from WASM client
#[derive(Deserialize)]
struct BridgeHello {
  page_url: Option<String>,
}

// === HTTP Handlers ===

const DEBUG_UI_HTML: &str = include_str!("debug_ui.html");

async fn get_ui() -> impl IntoResponse { Html(DEBUG_UI_HTML.to_string()) }

async fn get_status(State(state): State<Arc<BridgeState>>) -> impl IntoResponse {
  Json(bridge_status_payload(&state).await)
}

async fn get_windows(State(state): State<Arc<BridgeState>>) -> Response {
  match forward_http_request(&state, "GET", "windows", None).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn inspect_tree(
  State(state): State<Arc<BridgeState>>, Query(q): Query<LayoutQuery>,
) -> Response {
  let path = format!("inspect/tree?options={}", q.options.as_deref().unwrap_or("id"));
  match forward_http_request(&state, "GET", &path, None).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn inspect_widget(
  State(state): State<Arc<BridgeState>>, Path(id): Path<String>, Query(q): Query<LayoutQuery>,
) -> Response {
  let path = format!("inspect/{}?options={}", id, q.options.as_deref().unwrap_or("id"));
  match forward_http_request(&state, "GET", &path, None).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn add_overlay(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<OverlayRequest>,
) -> Response {
  match forward_http_request(&state, "POST", "overlay", Some(json!(payload))).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn remove_overlay(State(state): State<Arc<BridgeState>>, Path(id): Path<String>) -> Response {
  match forward_http_request(&state, "DELETE", &format!("overlay/{}", id), None).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn get_overlays(State(state): State<Arc<BridgeState>>) -> Response {
  match forward_http_request(&state, "GET", "overlays", None).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn clear_overlays(State(state): State<Arc<BridgeState>>) -> Response {
  match forward_http_request(&state, "DELETE", "overlays", None).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn capture_screenshot(State(state): State<Arc<BridgeState>>) -> Response {
  match forward_http_request(&state, "GET", "screenshot", None).await {
    Ok(body) => {
      // body contains the screenshot response with content_type and data
      if let (Some(content_type), Some(data)) = (
        body.get("content_type").and_then(|v| v.as_str()),
        body.get("data").and_then(|v| v.as_str()),
      ) {
        use base64::{Engine as _, engine::general_purpose};
        if let Ok(png_data) = general_purpose::STANDARD.decode(data) {
          return (StatusCode::OK, [("content-type", content_type)], png_data).into_response();
        }
      }
      (StatusCode::INTERNAL_SERVER_ERROR, "Failed to decode screenshot").into_response()
    }
    Err(e) => e.into_response(),
  }
}

async fn toggle_recording(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<RecordingRequest>,
) -> Response {
  match forward_http_request(&state, "POST", "recording", Some(json!(payload))).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn get_logs(
  State(state): State<Arc<BridgeState>>, Query(query): Query<LogsQuery>,
) -> Response {
  match forward_http_request(&state, "GET", &build_logs_path(&query), None).await {
    Ok(body) => {
      // Extract data from WASM response
      let data = body
        .get("data")
        .and_then(|v| v.as_str())
        .unwrap_or("");
      let dropped = body
        .get("dropped_total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

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

      (StatusCode::OK, headers, data.to_string()).into_response()
    }
    Err(e) => e.into_response(),
  }
}

async fn stream_logs(
  State(state): State<Arc<BridgeState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
  let state = state.clone();
  let cursor = Arc::new(Mutex::new(LogCursor::default()));

  let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_millis(3000)))
    .then(move |_| {
      let state = state.clone();
      let cursor = cursor.clone();
      async move {
        let mut events: Vec<Result<Event, std::convert::Infallible>> = Vec::new();

        let query = {
          let guard = cursor.lock().await;
          // Use from_seq for precise seq-based filtering when available
          LogsQuery {
            since_ts: None, // Not needed when using from_seq
            until_ts: None,
            limit: Some(200),
            from_seq: guard.last_seq.map(|seq| seq + 1),
          }
        };

        if let Ok(body) = forward_http_request(&state, "GET", &build_logs_path(&query), None).await
        {
          let data = body
            .get("data")
            .and_then(|v| v.as_str())
            .unwrap_or("");

          let raw_lines: Vec<&str> = data
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

          // Use server-provided max_seq for cursor update (more reliable than parsing
          // each line)
          let server_max_seq = body.get("max_seq").and_then(|v| v.as_u64());

          for line in &raw_lines {
            events.push(Ok(Event::default().event("log").data(*line)));
          }

          // Update cursor with server's max_seq
          if let Some(max_seq) = server_max_seq {
            let mut guard = cursor.lock().await;
            guard.last_seq = Some(max_seq);
          }
        }

        events.push(Ok(
          Event::default()
            .event("stats")
            .data(bridge_status_payload(&state).await.to_string()),
        ));

        futures::stream::iter(events)
      }
    })
    .flatten();

  Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

async fn set_logs_filter(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<LogsFilterRequest>,
) -> Response {
  match forward_http_request(&state, "POST", "logs/filter", Some(json!(payload))).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn inject_events(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<InjectEventsRequest>,
) -> Response {
  match forward_http_request(&state, "POST", "events/inject", Some(json!(payload))).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn start_event_macro_recording(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<StartEventMacroRecordingRequest>,
) -> Response {
  match forward_http_request(&state, "POST", "events/macro/start", Some(json!(payload))).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

async fn stop_event_macro_recording(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<StopEventMacroRecordingRequest>,
) -> Response {
  match forward_http_request(&state, "POST", "events/macro/stop", Some(json!(payload))).await {
    Ok(response) => Json(response).into_response(),
    Err(e) => e.into_response(),
  }
}

fn forward_timeout(method: &str, path: &str, body: Option<&Value>) -> std::time::Duration {
  if method.eq_ignore_ascii_case("POST") && path == "events/macro/start" {
    if let Some(duration_ms) = body
      .and_then(|body| body.get("duration_ms"))
      .and_then(Value::as_u64)
    {
      let timeout_ms = duration_ms
        .saturating_add(MACRO_START_TIMEOUT_GRACE_MS)
        .max(DEFAULT_FORWARD_TIMEOUT_MS);
      return std::time::Duration::from_millis(timeout_ms);
    }
  }

  std::time::Duration::from_millis(DEFAULT_FORWARD_TIMEOUT_MS)
}

async fn capture_start(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<CaptureStartRequest>,
) -> Response {
  let output_dir = payload.output_dir.clone();
  let remote_payload = json!({
    "include": payload.include,
    "pre_ms": payload.pre_ms,
    "post_ms": payload.post_ms,
  });

  match forward_http_request(&state, "POST", "capture/start", Some(remote_payload)).await {
    Ok(response) => match serde_json::from_value::<RemoteCaptureStartResponse>(response) {
      Ok(remote) => {
        let capture_dir = capture_dir_from_root(output_dir, &remote.capture_id);
        let capture_dir_abs = absolutize_path(&capture_dir);
        let create_dir = capture_dir_abs.clone();
        let create_result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
          std::fs::create_dir_all(create_dir.join("frames"))?;
          Ok(())
        })
        .await;
        match create_result {
          Ok(Ok(())) => {}
          Ok(Err(err)) => {
            return BridgeError::PersistError(format!(
              "Failed to prepare capture directory: {err}"
            ))
            .into_response();
          }
          Err(_) => {
            return BridgeError::PersistError("Failed to prepare capture directory.".into())
              .into_response();
          }
        }

        let session = BridgeCaptureSession {
          capture_id: remote.capture_id.clone(),
          capture_dir: capture_dir_abs.clone(),
        };
        *state.active_capture.lock().await = Some(session);

        Json(json!({
          "capture_id": remote.capture_id,
          "capture_dir": capture_dir_abs.to_string_lossy().to_string(),
        }))
        .into_response()
      }
      Err(_) => {
        BridgeError::HttpError(500, "Invalid capture start response".into()).into_response()
      }
    },
    Err(e) => e.into_response(),
  }
}

async fn capture_stop(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<CaptureStopRequest>,
) -> Response {
  let requested_capture_id = payload.capture_id.clone();
  match forward_http_request(&state, "POST", "capture/stop", Some(json!(&payload))).await {
    Ok(response) => match serde_json::from_value::<RemoteCaptureBundle>(response) {
      Ok(bundle) => {
        let capture_dir =
          take_capture_dir(&state, requested_capture_id.as_deref(), &bundle.capture_id)
            .await
            .unwrap_or_else(|| capture_dir_from_root(None, &bundle.capture_id));
        match persist_capture_bundle(capture_dir, bundle).await {
          Ok(resp) => Json(resp).into_response(),
          Err(e) => e.into_response(),
        }
      }
      Err(_) => {
        BridgeError::HttpError(500, "Invalid capture bundle response".into()).into_response()
      }
    },
    Err(e) => e.into_response(),
  }
}

async fn capture_one_shot(
  State(state): State<Arc<BridgeState>>, Json(payload): Json<CaptureOneShotRequest>,
) -> Response {
  let capture_dir_root = payload.output_dir.clone();
  let remote_payload = json!({
    "include": payload.include,
    "pre_ms": payload.pre_ms,
    "post_ms": payload.post_ms,
    "settle_ms": payload.settle_ms,
  });

  match forward_http_request(&state, "POST", "capture/one_shot", Some(remote_payload)).await {
    Ok(response) => match serde_json::from_value::<RemoteCaptureBundle>(response) {
      Ok(bundle) => {
        let capture_dir = capture_dir_from_root(capture_dir_root, &bundle.capture_id);
        match persist_capture_bundle(capture_dir, bundle).await {
          Ok(resp) => Json(resp).into_response(),
          Err(e) => e.into_response(),
        }
      }
      Err(_) => {
        BridgeError::HttpError(500, "Invalid capture bundle response".into()).into_response()
      }
    },
    Err(e) => e.into_response(),
  }
}

async fn take_capture_dir(
  state: &Arc<BridgeState>, requested_id: Option<&str>, actual_id: &str,
) -> Option<PathBuf> {
  let mut guard = state.active_capture.lock().await;
  match guard.as_ref() {
    Some(session)
      if requested_id
        .map(|id| id == session.capture_id.as_str())
        .unwrap_or(true)
        && session.capture_id == actual_id =>
    {
      guard.take().map(|session| session.capture_dir)
    }
    _ => None,
  }
}

// === Request Forwarding ===

#[derive(Debug)]
enum BridgeError {
  NoSession,
  SessionDisconnected,
  EncodeError,
  ChannelClosed,
  Timeout,
  HttpError(u16, String),
  PersistError(String),
}

impl IntoResponse for BridgeError {
  fn into_response(self) -> Response {
    let (status, msg): (StatusCode, String) = match self {
      BridgeError::NoSession => (
        StatusCode::SERVICE_UNAVAILABLE,
        "No WASM debug session connected. Open the browser page with ?ribir_debug_server=<ws-url>."
          .to_string(),
      ),
      BridgeError::SessionDisconnected => {
        (StatusCode::SERVICE_UNAVAILABLE, "WASM debug session disconnected.".to_string())
      }
      BridgeError::EncodeError => {
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to encode request.".to_string())
      }
      BridgeError::ChannelClosed => {
        (StatusCode::INTERNAL_SERVER_ERROR, "Response channel closed.".to_string())
      }
      BridgeError::Timeout => (StatusCode::REQUEST_TIMEOUT, "Request timed out.".to_string()),
      BridgeError::HttpError(code, msg) => {
        (StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), msg)
      }
      BridgeError::PersistError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
    };
    (status, msg).into_response()
  }
}

async fn forward_http_request(
  state: &Arc<BridgeState>, method: &str, path: &str, body: Option<Value>,
) -> Result<Value, BridgeError> {
  let timeout = forward_timeout(method, path, body.as_ref());
  let request_id = state
    .next_request_id
    .fetch_add(1, Ordering::Relaxed);

  let session = {
    let active = state.active.lock().await;
    active
      .as_ref()
      .map(|a| (a.token, a.sender.clone()))
  };

  let Some((token, sender)) = session else {
    return Err(BridgeError::NoSession);
  };

  let request =
    BridgeRequest { request_id, path: path.to_string(), method: method.to_string(), body };
  let text = serde_json::to_string(&request).map_err(|_| BridgeError::EncodeError)?;

  let (tx, rx) = oneshot::channel();
  state
    .pending
    .lock()
    .await
    .insert(request_id, PendingRequest { token, reply: tx });

  if sender.send(Message::Text(text.into())).is_err() {
    state.pending.lock().await.remove(&request_id);
    return Err(BridgeError::SessionDisconnected);
  }

  match tokio::time::timeout(timeout, rx).await {
    Ok(Ok(response)) => {
      // Parse response
      if let Ok(response) = serde_json::from_value::<BridgeResponse>(response) {
        if response.status >= 400 {
          let msg = response
            .body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
          return Err(BridgeError::HttpError(response.status, msg.to_string()));
        }
        Ok(response.body)
      } else {
        Err(BridgeError::HttpError(500, "Invalid response format".to_string()))
      }
    }
    Ok(Err(_)) => Err(BridgeError::ChannelClosed),
    Err(_) => {
      state.pending.lock().await.remove(&request_id);
      Err(BridgeError::Timeout)
    }
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::forward_timeout;

  #[test]
  fn macro_start_timeout_tracks_requested_duration() {
    assert_eq!(
      forward_timeout("POST", "events/macro/start", Some(&json!({ "duration_ms": 30_000 }))),
      std::time::Duration::from_millis(35_000)
    );
  }

  #[test]
  fn non_macro_requests_keep_default_timeout() {
    assert_eq!(forward_timeout("GET", "status", None), std::time::Duration::from_millis(15_000));
  }
}

async fn bridge_status_payload(state: &Arc<BridgeState>) -> Value {
  let (session_connected, page_url) = {
    let active = state.active.lock().await;
    active
      .as_ref()
      .map(|a| (true, a.page_url.clone()))
      .unwrap_or((false, None))
  };
  let pending_requests = state.pending.lock().await.len();
  let local_capture = state.active_capture.lock().await.clone();
  let capture_root = bridge_capture_root();

  let mut payload = if session_connected {
    forward_http_request(state, "GET", "status", None)
      .await
      .unwrap_or_else(|_| json!({}))
  } else {
    json!({})
  };

  let Some(obj) = payload.as_object_mut() else {
    return json!({
      "mode": "wasm_bridge",
      "session_connected": session_connected,
      "page_url": page_url,
      "pending_requests": pending_requests,
      "recording": false,
      "log_sink_connected": false,
      "filter_reload_installed": false,
      "filter": null,
      "dropped_total": 0,
      "ring_len": 0,
      "capture_root": capture_root.to_string_lossy().to_string(),
      "active_capture": local_capture.as_ref().map(|session| json!({
        "capture_id": session.capture_id,
        "capture_dir": session.capture_dir.to_string_lossy().to_string(),
      })),
      "active_macro_recording": null,
    });
  };

  obj.insert("mode".into(), json!("wasm_bridge"));
  obj.insert("session_connected".into(), json!(session_connected));
  obj.insert("page_url".into(), json!(page_url));
  obj.insert("pending_requests".into(), json!(pending_requests));
  obj.insert("capture_root".into(), json!(capture_root.to_string_lossy().to_string()));
  obj
    .entry("recording")
    .or_insert_with(|| json!(false));
  obj
    .entry("log_sink_connected")
    .or_insert_with(|| json!(false));
  obj
    .entry("filter_reload_installed")
    .or_insert_with(|| json!(false));
  obj.entry("filter").or_insert(Value::Null);
  obj
    .entry("dropped_total")
    .or_insert_with(|| json!(0));
  obj.entry("ring_len").or_insert_with(|| json!(0));
  obj
    .entry("active_macro_recording")
    .or_insert(Value::Null);

  if let Some(session) = local_capture {
    match obj.get_mut("active_capture") {
      Some(Value::Object(active_capture)) => {
        active_capture.insert("capture_id".into(), json!(session.capture_id));
        active_capture
          .insert("capture_dir".into(), json!(session.capture_dir.to_string_lossy().to_string()));
      }
      _ => {
        obj.insert(
          "active_capture".into(),
          json!({
            "capture_id": session.capture_id,
            "capture_dir": session.capture_dir.to_string_lossy().to_string(),
          }),
        );
      }
    }
  }

  payload
}

fn build_logs_path(query: &LogsQuery) -> String {
  let mut params = Vec::new();
  if let Some(since_ts) = query.since_ts {
    params.push(format!("since_ts={since_ts}"));
  }
  if let Some(until_ts) = query.until_ts {
    params.push(format!("until_ts={until_ts}"));
  }
  if let Some(limit) = query.limit {
    params.push(format!("limit={limit}"));
  }
  if let Some(from_seq) = query.from_seq {
    params.push(format!("from_seq={from_seq}"));
  }
  if params.is_empty() { "logs".to_string() } else { format!("logs?{}", params.join("&")) }
}

fn bridge_capture_root() -> PathBuf {
  std::env::var("RIBIR_CAPTURE_DIR")
    .ok()
    .filter(|value| !value.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("captures"))
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

fn capture_dir_from_root(output_dir: Option<String>, capture_id: &str) -> PathBuf {
  let root = output_dir
    .filter(|value| !value.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(bridge_capture_root);
  absolutize_path(root.join(capture_id))
}

async fn persist_capture_bundle(
  capture_dir: PathBuf, bundle: RemoteCaptureBundle,
) -> Result<CaptureStopResponse, BridgeError> {
  let capture_dir = absolutize_path(capture_dir);
  let manifest_path = capture_dir.join("manifest.json");
  let capture_dir_str = capture_dir.to_string_lossy().to_string();
  let manifest_path_str = manifest_path.to_string_lossy().to_string();
  let bundle_for_write = bundle.clone();

  tokio::task::spawn_blocking(move || -> std::io::Result<()> {
    use base64::{Engine as _, engine::general_purpose};

    std::fs::create_dir_all(capture_dir.join("frames"))?;

    if let Some(logs) = bundle_for_write.logs.as_ref() {
      let mut file = std::fs::File::create(capture_dir.join("logs.jsonl"))?;
      for line in &logs.lines {
        std::io::Write::write_all(&mut file, line.as_bytes())?;
        std::io::Write::write_all(&mut file, b"\n")?;
      }
    }

    if let Some(frames) = bundle_for_write.frames.as_ref() {
      for frame in frames {
        let Some(data) = frame.png_base64.as_ref() else {
          continue;
        };
        let bytes = general_purpose::STANDARD
          .decode(data)
          .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        let abs_path = capture_dir.join(&frame.path);
        std::fs::create_dir_all(
          abs_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
        )?;
        std::fs::write(abs_path, bytes)?;
      }
    }

    let manifest = serde_json::json!({
      "schema_version": 1,
      "capture_id": bundle_for_write.capture_id,
      "start_ts_unix_ms": bundle_for_write.start_ts_unix_ms,
      "end_ts_unix_ms": bundle_for_write.end_ts_unix_ms,
      "options": {
        "include_logs": bundle_for_write.options.include_logs,
        "include_images": bundle_for_write.options.include_images,
        "pre_ms": bundle_for_write.options.pre_ms,
        "post_ms": bundle_for_write.options.post_ms,
        "filter_at_start": bundle_for_write.filter_at_start,
      },
      "logs": bundle_for_write.logs.as_ref().map(|logs| {
        serde_json::json!({
          "path": "logs.jsonl",
          "count": logs.lines.len(),
          "dropped_total": logs.dropped_total,
          "since_ts_unix_ms": logs.since_ts_unix_ms,
          "until_ts_unix_ms": logs.until_ts_unix_ms,
        })
      }),
      "frames": bundle_for_write.frames.as_ref().map(|frames| {
        frames
          .iter()
          .map(|frame| serde_json::json!({
            "seq": frame.seq,
            "ts_unix_ms": frame.ts_unix_ms,
            "path": frame.path,
          }))
          .collect::<Vec<_>>()
      }),
    });
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest).unwrap_or_default())
  })
  .await
  .map_err(|_| BridgeError::PersistError("Failed to persist capture bundle.".into()))?
  .map_err(|err| BridgeError::PersistError(format!("Failed to persist capture bundle: {err}")))?;

  Ok(CaptureStopResponse {
    capture_id: bundle.capture_id,
    capture_dir: capture_dir_str,
    manifest_path: manifest_path_str,
  })
}

// === WebSocket Handler ===

async fn ws_handler(
  State(state): State<Arc<BridgeState>>, ws: WebSocketUpgrade,
) -> impl IntoResponse {
  ws.on_upgrade(move |socket| bridge_session(socket, state))
}

async fn bridge_session(socket: WebSocket, state: Arc<BridgeState>) {
  let token = state
    .next_session_token
    .fetch_add(1, Ordering::Relaxed);
  let (mut ws_tx, mut ws_rx) = socket.split();

  let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

  // Send hello
  {
    let active = ActiveSession { token, sender: tx.clone(), page_url: None };
    *state.active.lock().await = Some(active);
    tracing::info!("WASM debug session connected (token={})", token);
  }
  *state.active_capture.lock().await = None;

  // Spawn task to forward messages to WebSocket
  let send_task = tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
      if ws_tx.send(msg).await.is_err() {
        break;
      }
    }
  });

  // Receive messages from WebSocket
  while let Some(msg) = ws_rx.next().await {
    match msg {
      Ok(Message::Text(text)) => {
        // Try to parse as response first
        if let Ok(response) = serde_json::from_str::<BridgeResponse>(&text) {
          // This is a response to a pending request
          if let Some(pending) = state
            .pending
            .lock()
            .await
            .remove(&response.request_id)
          {
            let _ = pending
              .reply
              .send(serde_json::to_value(response).unwrap_or_default());
          }
        } else if let Ok(hello) = serde_json::from_str::<BridgeHello>(&text) {
          // Hello message from WASM client
          let mut active = state.active.lock().await;
          if let Some(session) = active.as_mut() {
            session.page_url = hello.page_url;
          }
        } else if let Ok(response) = serde_json::from_str::<Value>(&text) {
          // Handle as generic response with request_id
          if let Some(request_id) = response
            .get("request_id")
            .and_then(|v| v.as_u64())
          {
            if let Some(pending) = state.pending.lock().await.remove(&request_id) {
              let _ = pending.reply.send(response);
            }
          }
        }
      }
      Ok(Message::Close(_)) => break,
      Err(e) => {
        tracing::warn!("WebSocket error: {}", e);
        break;
      }
      _ => {}
    }
  }

  // Cleanup
  send_task.abort();
  let should_clear = {
    let mut active = state.active.lock().await;
    if active.as_ref().is_some_and(|a| a.token == token) {
      *active = None;
      true
    } else {
      false
    }
  };
  if should_clear {
    *state.active_capture.lock().await = None;
    tracing::info!("WASM debug session disconnected (token={})", token);
  }
}

// === Query Types ===

#[derive(Deserialize, Default)]
struct LayoutQuery {
  #[serde(default)]
  options: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct OverlayRequest {
  #[serde(default)]
  window_id: Option<String>,
  id: String,
  color: String,
}

#[derive(Deserialize, Serialize)]
struct RecordingRequest {
  enable: bool,
}

#[derive(Deserialize, Default)]
struct LogsQuery {
  since_ts: Option<u64>,
  until_ts: Option<u64>,
  limit: Option<usize>,
  /// Start from this sequence number (inclusive).
  from_seq: Option<u64>,
}

#[derive(Default)]
struct LogCursor {
  /// Monotonic sequence number cursor for precise dedup.
  last_seq: Option<u64>,
}

#[derive(Deserialize, Serialize)]
struct LogsFilterRequest {
  filter: String,
}

#[derive(Deserialize, Serialize)]
struct InjectEventsRequest {
  #[serde(default)]
  window_id: Option<String>,
  events: Vec<Value>,
}

// === Event Macro Recording Types ===

#[derive(Deserialize, Serialize)]
struct StartEventMacroRecordingRequest {
  #[serde(default)]
  window_id: Option<String>,
  #[serde(default)]
  duration_ms: Option<u64>,
}

#[derive(Deserialize, Serialize)]
struct StopEventMacroRecordingRequest {
  #[serde(default)]
  recording_id: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CaptureStartRequest {
  include: Vec<String>,
  #[serde(default)]
  pre_ms: Option<u64>,
  #[serde(default)]
  post_ms: Option<u64>,
  #[serde(default)]
  output_dir: Option<String>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct CaptureStopRequest {
  #[serde(default)]
  capture_id: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CaptureOneShotRequest {
  include: Vec<String>,
  #[serde(default)]
  pre_ms: Option<u64>,
  #[serde(default)]
  post_ms: Option<u64>,
  #[serde(default)]
  settle_ms: Option<u64>,
  #[serde(default)]
  output_dir: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct CaptureStopResponse {
  capture_id: String,
  capture_dir: String,
  manifest_path: String,
}

#[derive(Deserialize)]
struct RemoteCaptureStartResponse {
  capture_id: String,
}

#[derive(Clone, Deserialize)]
struct RemoteCaptureBundle {
  capture_id: String,
  start_ts_unix_ms: u64,
  end_ts_unix_ms: u64,
  options: RemoteCaptureOptions,
  #[serde(default)]
  filter_at_start: Option<String>,
  #[serde(default)]
  logs: Option<RemoteCaptureLogsPayload>,
  #[serde(default)]
  frames: Option<Vec<RemoteCaptureFrameEntry>>,
}

#[derive(Clone, Deserialize)]
struct RemoteCaptureOptions {
  include_logs: bool,
  include_images: bool,
  pre_ms: u64,
  post_ms: u64,
}

#[derive(Clone, Deserialize)]
struct RemoteCaptureLogsPayload {
  lines: Vec<String>,
  dropped_total: u64,
  since_ts_unix_ms: u64,
  until_ts_unix_ms: u64,
}

#[derive(Clone, Deserialize)]
struct RemoteCaptureFrameEntry {
  seq: u64,
  ts_unix_ms: u64,
  path: String,
  #[serde(default)]
  png_base64: Option<String>,
}
