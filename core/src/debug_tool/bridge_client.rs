//! Universal Debug Bridge Client.
//!
//! Connects to the Bridge Server (CLI) via WebSocket:
//! - **Native**: Uses tokio-tungstenite WebSocket client (auto-connects to
//!   RIBIR_DEBUG_URL)
//! - **WASM**: Uses web_sys::WebSocket (via ribir_debug_server query param)
//!
//! Architecture: Both Native and WASM apps act as clients connecting to CLI
//! server. This eliminates duplicate HTTP server code and simplifies
//! maintenance.
//!
//! ## Usage
//!
//! **Native**: Auto-connects to `http://127.0.0.1:2333` by default (converted to ws://)
//! ```bash
//! cargo run --features debug
//! ```
//!
//! **WASM**: Pass `ribir_debug_server` query parameter (HTTP URL,
//! auto-converted to WebSocket) ```
//! http://localhost:8080/?ribir_debug_server=http://127.0.0.1:2333
//! ```

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::debug_tool::{
  runtime::{
    CaptureOneShotRequest, CaptureStartRequest, CaptureStopRequest, DebugServerState,
    build_status_response, capture_one_shot_bundle_inner, capture_start_inner,
    capture_stop_bundle_inner, start_debug_runtime,
  },
  service::*,
  types::DebugCommand,
};

/// Request from Bridge Server to client.
#[derive(Deserialize)]
pub struct BridgeRequest {
  pub request_id: u64,
  pub path: String,
  pub method: String,
  pub body: Option<Value>,
}

/// Response from client to Bridge Server.
#[derive(Serialize)]
pub struct BridgeResponse {
  pub request_id: u64,
  pub status: u16,
  pub body: Value,
}

/// Hello message from client to Bridge Server.
#[derive(Serialize)]
pub struct BridgeHello {
  /// Client type: "native" or "wasm"
  pub client_type: String,
  /// Optional metadata (PID for native, page URL for WASM)
  pub metadata: Option<String>,
}

/// Start the debug bridge client.
///
/// Works for both Native and WASM targets. Connects to CLI bridge server
/// and handles debug requests over WebSocket.
pub fn start_debug_client() -> tokio::sync::mpsc::Sender<DebugCommand> {
  let state = start_debug_runtime();
  let bridge_url = get_bridge_url();

  if let Some(url) = bridge_url {
    connect_bridge(url, state.clone());
  } else {
    tracing::warn!("Debug bridge URL not specified. Using default: ws://127.0.0.1:2333/ws");
  }

  state.command_tx.clone()
}

/// Get bridge URL from environment (native) or query params (WASM).
/// Converts HTTP URL to WebSocket URL.
#[cfg(not(target_arch = "wasm32"))]
fn get_bridge_url() -> Option<String> {
  let http_url = std::env::var("RIBIR_DEBUG_URL")
    .ok()
    .unwrap_or_else(|| "http://127.0.0.1:2333".to_string());

  // Convert http:// to ws:// and add /ws path
  let ws_url = if let Some(host) = http_url.strip_prefix("https://") {
    format!("wss://{}/ws", host)
  } else if let Some(host) = http_url.strip_prefix("http://") {
    format!("ws://{}/ws", host)
  } else if http_url.starts_with("ws://") || http_url.starts_with("wss://") {
    http_url
  } else {
    format!("ws://{}/ws", http_url)
  };

  Some(ws_url)
}

#[cfg(target_arch = "wasm32")]
fn get_bridge_url() -> Option<String> { bridge_url_from_query() }

/// Connect to bridge server (Native implementation).
#[cfg(not(target_arch = "wasm32"))]
fn connect_bridge(url: String, state: Arc<DebugServerState>) {
  use futures::{SinkExt, StreamExt};
  use tokio::sync::mpsc;
  use tokio_tungstenite::{connect_async, tungstenite::Message};

  tokio::spawn(async move {
    let (ws_stream, response) = match connect_async(&url).await {
      Ok(conn) => conn,
      Err(e) => {
        tracing::warn!("Failed to connect to debug bridge: {}", e);
        return;
      }
    };

    tracing::info!("Connected to debug bridge (HTTP {})", response.status());

    let (mut write, mut read) = ws_stream.split();

    // Send hello
    let hello = BridgeHello {
      client_type: "native".to_string(),
      metadata: Some(std::process::id().to_string()),
    };

    if let Err(e) = write
      .send(Message::Text(serde_json::to_string(&hello).unwrap()))
      .await
    {
      tracing::warn!("Failed to send bridge hello: {}", e);
      return;
    }

    // Create channel for sending responses
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Writer task: send responses back to bridge
    let writer_task = tokio::spawn(async move {
      while let Some(text) = rx.recv().await {
        if let Err(e) = write.send(Message::Text(text)).await {
          tracing::warn!("Failed to send bridge response: {}", e);
          break;
        }
      }
    });

    // Handle messages
    while let Some(msg) = read.next().await {
      match msg {
        Ok(Message::Text(text)) => {
          let state = state.clone();
          let tx = tx.clone();

          tokio::spawn(async move {
            if let Ok(request) = serde_json::from_str::<BridgeRequest>(&text) {
              let response = handle_request(request, &state).await;
              if let Ok(text) = serde_json::to_string(&response) {
                let _ = tx.send(text);
              }
            }
          });
        }
        Ok(Message::Close(_)) | Err(_) => break,
        _ => {}
      }
    }

    writer_task.abort();
    tracing::info!("Debug bridge connection lost");
  });
}

/// Connect to bridge server (WASM implementation).
#[cfg(target_arch = "wasm32")]
fn connect_bridge(url: String, state: Arc<DebugServerState>) {
  use web_sys::wasm_bindgen::{JsCast, closure::Closure};

  let Ok(ws) = web_sys::WebSocket::new(&url) else {
    tracing::warn!("Failed to create WebSocket for debug bridge");
    return;
  };

  // On open: send hello and start stats forwarding
  let ws_open = ws.clone();
  let state_open = state.clone();
  let onopen = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
    let ws_for_send = ws_open.clone();
    let hello = BridgeHello { client_type: "wasm".to_string(), metadata: current_page_url() };
    let _ = ws_for_send.send_with_str(&serde_json::to_string(&hello).unwrap());
    start_stats_forwarding(state_open.clone());
  });
  ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
  onopen.forget();

  // On message: handle request
  let ws_msg = ws.clone();
  let state_msg = state.clone();
  let onmessage =
    Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
      let Some(text) = ev.data().as_string() else { return };
      let ws = ws_msg.clone();
      let state = state_msg.clone();
      wasm_bindgen_futures::spawn_local(async move {
        if let Ok(req) = serde_json::from_str::<BridgeRequest>(&text) {
          let resp = handle_request(req, &state).await;
          let _ = ws.send_with_str(&serde_json::to_string(&resp).unwrap());
        }
      });
    });
  ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
  onmessage.forget();

  // On error/close
  ws.set_onerror(Some(
    Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
      tracing::warn!("Debug bridge WebSocket error");
    })
    .as_ref()
    .unchecked_ref(),
  ));
  ws.set_onclose(Some(
    Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
      tracing::warn!("Debug bridge WebSocket closed");
    })
    .as_ref()
    .unchecked_ref(),
  ));
}

/// Handle a bridge request.
async fn handle_request(request: BridgeRequest, state: &Arc<DebugServerState>) -> BridgeResponse {
  let BridgeRequest { request_id, path, method, body } = request;
  let path = path.trim_start_matches('/');

  let (status, body) = match (method.as_str(), path) {
    ("GET", "status") => (200, serde_json::to_value(build_status_response(state).await).unwrap()),
    ("GET", "windows") => match get_windows_svc(state).await {
      Ok(w) => (200, serde_json::to_value(w).unwrap()),
      Err(_) => (500, json_error("Failed to get windows")),
    },
    ("GET", p) if p.starts_with("inspect/tree") => {
      let opts = parse_query_options(path);
      match inspect_tree_svc(state, None, opts).await {
        Ok(t) => (200, t),
        Err(_) => (500, json_error("Failed to inspect tree")),
      }
    }
    ("GET", p) if p.starts_with("inspect/") => {
      let id = p
        .strip_prefix("inspect/")
        .unwrap()
        .split('?')
        .next()
        .unwrap();
      let opts = parse_query_options(path);
      match inspect_widget_svc(state, None, id.into(), opts).await {
        Ok(i) => (200, i),
        Err(ServiceError::NotFound) => (404, json_error("Widget not found")),
        Err(_) => (500, json_error("Failed to inspect widget")),
      }
    }
    ("GET", "screenshot") => match capture_screenshot_svc(state).await {
      Ok(img) => {
        let mut data = Vec::new();
        if img.write_as_webp(&mut data).is_ok() {
          use base64::{Engine as _, engine::general_purpose};
          (
            200,
            serde_json::json!({ "content_type": "image/png", "data": general_purpose::STANDARD.encode(&data) }),
          )
        } else {
          (500, json_error("Failed to encode image"))
        }
      }
      Err(_) => (500, json_error("Failed to capture screenshot")),
    },
    ("POST", "overlay") => {
      let b = body.unwrap_or_default();
      let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
      let color = b
        .get("color")
        .and_then(|v| v.as_str())
        .unwrap_or("#FF000080");
      match add_overlay_svc(state, None, id.into(), color.into()).await {
        Ok(()) => (200, serde_json::json!({ "ok": true })),
        Err(_) => (500, json_error("Failed to add overlay")),
      }
    }
    ("DELETE", p) if p.starts_with("overlay/") => {
      let id = p.strip_prefix("overlay/").unwrap();
      match remove_overlay_svc(state, None, id.into()).await {
        Ok(()) => (200, serde_json::json!({ "ok": true })),
        Err(_) => (500, json_error("Failed to remove overlay")),
      }
    }
    ("DELETE", "overlays") => {
      let _ = clear_overlays_svc(state, None).await;
      (200, serde_json::json!({ "ok": true }))
    }
    ("GET", "overlays") => {
      use crate::debug_tool::service::get_windows_svc;
      let overlays = match get_windows_svc(state).await {
        Ok(windows) if !windows.is_empty() => {
          let window_id = windows[0].id;
          get_overlays_svc(window_id)
        }
        _ => Vec::new(),
      };
      let result: Vec<_> = overlays
        .into_iter()
        .map(|(id, color)| serde_json::json!([id, color]))
        .collect();
      (200, serde_json::to_value(result).unwrap())
    }
    ("POST", "events/inject") => {
      let b = body.unwrap_or_default();
      match serde_json::from_value::<crate::debug_tool::types::InjectEventsRequest>(b) {
        Ok(req) => match inject_events_svc(state, None, req.events).await {
          Ok(r) => (200, serde_json::to_value(r).unwrap()),
          Err(e) => (400, json_error(&e.to_string())),
        },
        Err(e) => (400, json_error(&e.to_string())),
      }
    }
    ("POST", "events/macro/start") => {
      let b = body.unwrap_or_default();
      match serde_json::from_value::<crate::debug_tool::types::StartEventMacroRecordingRequest>(b) {
        Ok(req) => {
          let (tx, rx) = tokio::sync::oneshot::channel();
          if state
            .command_tx
            .send(crate::debug_tool::types::DebugCommand::StartEventMacroRecording {
              window_id: req.window_id,
              duration_ms: req.duration_ms,
              reply: tx,
            })
            .await
            .is_ok()
          {
            match rx.await {
              Ok(Ok(r)) => (200, serde_json::to_value(r).unwrap()),
              Ok(Err(e)) => (400, json_error(&e)),
              Err(_) => (500, json_error("Internal error")),
            }
          } else {
            (500, json_error("Internal error"))
          }
        }
        Err(e) => (400, json_error(&e.to_string())),
      }
    }
    ("POST", "events/macro/stop") => {
      let b = body.unwrap_or_default();
      match serde_json::from_value::<crate::debug_tool::types::StopEventMacroRecordingRequest>(b) {
        Ok(req) => {
          let (tx, rx) = tokio::sync::oneshot::channel();
          if state
            .command_tx
            .send(crate::debug_tool::types::DebugCommand::StopEventMacroRecording {
              recording_id: req.recording_id,
              reply: tx,
            })
            .await
            .is_ok()
          {
            match rx.await {
              Ok(Ok(r)) => (200, serde_json::to_value(r).unwrap()),
              Ok(Err(e)) => (400, json_error(&e)),
              Err(_) => (500, json_error("Internal error")),
            }
          } else {
            (500, json_error("Internal error"))
          }
        }
        Err(e) => (400, json_error(&e.to_string())),
      }
    }
    ("POST", "capture/start") => {
      let b = body.unwrap_or_default();
      match serde_json::from_value::<CaptureStartRequest>(b) {
        Ok(req) => match capture_start_inner(
          state.clone(),
          req.include,
          req.pre_ms.unwrap_or(2_000),
          req.post_ms.unwrap_or(1_000),
          None,
        )
        .await
        {
          Ok(r) => (200, serde_json::to_value(r).unwrap()),
          Err(e) => capture_error(e),
        },
        Err(e) => (400, json_error(&e.to_string())),
      }
    }
    ("POST", "capture/stop") => {
      let b = body.unwrap_or_default();
      match serde_json::from_value::<CaptureStopRequest>(b) {
        Ok(req) => match capture_stop_bundle_inner(state.clone(), req).await {
          Ok(r) => (200, serde_json::to_value(r).unwrap()),
          Err(e) => capture_error(e),
        },
        Err(e) => (400, json_error(&e.to_string())),
      }
    }
    ("POST", "capture/one_shot") => {
      let b = body.unwrap_or_default();
      match serde_json::from_value::<CaptureOneShotRequest>(b) {
        Ok(req) => match capture_one_shot_bundle_inner(state.clone(), req).await {
          Ok(r) => (200, serde_json::to_value(r).unwrap()),
          Err(e) => capture_error(e),
        },
        Err(e) => (400, json_error(&e.to_string())),
      }
    }
    ("GET", p) if p == "logs" || p.starts_with("logs?") => {
      let (since, until, limit, from_seq) = parse_logs_query(path);
      let (lines, max_ts, ring_len, max_seq) = {
        let ring = state.log_ring.lock().await;
        if let Some(seq) = from_seq {
          // Use seq-based query for precise streaming
          let seq_lines = ring.query_lines_from_seq(seq, limit);
          let lines: Vec<Arc<str>> = seq_lines
            .iter()
            .map(|(_, line)| line.clone())
            .collect();
          let max_seq = seq_lines.iter().map(|(seq, _)| *seq).max();
          (lines, ring.max_ts(), ring.len(), max_seq)
        } else {
          (ring.query_lines(since, until, limit), ring.max_ts(), ring.len(), ring.max_seq())
        }
      };
      let mut out = String::new();
      for line in lines {
        out.push_str(line.as_ref());
        out.push('\n');
      }
      let mut resp = serde_json::json!({
        "content_type": "application/x-ndjson",
        "data": out,
        "dropped_total": crate::logging::dropped_logs_total(),
        "ring_len": ring_len,
      });
      if let Some(ts) = max_ts {
        resp["max_ts"] = serde_json::json!(ts);
      }
      if let Some(seq) = max_seq {
        resp["max_seq"] = serde_json::json!(seq);
      }
      (200, resp)
    }
    ("POST", "logs/filter") => {
      let filter = body
        .and_then(|b| {
          b.get("filter")
            .and_then(|v| v.as_str().map(String::from))
        })
        .unwrap_or_default();
      match crate::logging::update_filter(&filter) {
        Ok(()) => (200, serde_json::json!({ "ok": true })),
        Err(e) => (400, json_error(&e)),
      }
    }
    _ => (404, json_error(&format!("Unknown endpoint: {} {}", method, path))),
  };

  BridgeResponse { request_id, status, body }
}

fn json_error(msg: &str) -> Value { serde_json::json!({ "error": msg }) }

fn capture_error(err: crate::debug_tool::runtime::CaptureError) -> (u16, Value) {
  match err {
    crate::debug_tool::runtime::CaptureError::Conflict => {
      (409, json_error("Capture already in progress"))
    }
    crate::debug_tool::runtime::CaptureError::Timeout => (408, json_error("Capture timeout")),
    crate::debug_tool::runtime::CaptureError::NotFound => {
      (404, json_error("No active capture found"))
    }
    crate::debug_tool::runtime::CaptureError::Internal => (500, json_error("Failed to capture")),
  }
}

fn parse_query_options(path: &str) -> crate::debug_tool::types::InspectOptions {
  let mut opts = crate::debug_tool::types::InspectOptions::default();
  if let Some(query) = path.split('?').nth(1) {
    for pair in query.split('&') {
      if let Some((key, value)) = pair.split_once('=')
        && key == "options"
      {
        opts = crate::debug_tool::helpers::parse_inspect_options(Some(value));
      }
    }
  }
  opts
}

fn parse_logs_query(path: &str) -> (Option<u64>, Option<u64>, Option<usize>, Option<u64>) {
  let (mut since, mut until, mut limit, mut from_seq) = (None, None, None, None);
  if let Some(query) = path.split('?').nth(1) {
    for pair in query.split('&') {
      if let Some((k, v)) = pair.split_once('=') {
        match k {
          "since_ts" => since = v.parse().ok(),
          "until_ts" => until = v.parse().ok(),
          "limit" => limit = v.parse().ok(),
          "from_seq" => from_seq = v.parse().ok(),
          _ => {}
        }
      }
    }
  }
  (since, until, limit, from_seq)
}

#[cfg(target_arch = "wasm32")]
fn start_stats_forwarding(state: Arc<DebugServerState>) {
  use web_sys::wasm_bindgen::{JsCast, closure::Closure};
  let window = web_sys::window().unwrap();
  let cb = Closure::<dyn FnMut()>::new(move || {
    let state = state.clone();
    wasm_bindgen_futures::spawn_local(async move {
      let ring_len = state.log_ring.lock().await.len();
      tracing::debug!("WASM debug stats: ring_len={}", ring_len);
    });
  });
  let _ = window.set_interval_with_callback_and_timeout_and_arguments(
    cb.as_ref().unchecked_ref(),
    2000,
    &js_sys::Array::new(),
  );
  cb.forget();
}

#[cfg(target_arch = "wasm32")]
fn bridge_url_from_query() -> Option<String> {
  let window = web_sys::window()?;
  let location = window.location();
  let search = location.search().ok()?;
  let query = search.strip_prefix('?').unwrap_or(&search);
  let http_url = url::form_urlencoded::parse(query.as_bytes())
    .find(|(k, _)| k == "ribir_debug_server")
    .map(|(_, v)| v.into_owned())
    .filter(|v| !v.trim().is_empty())?;

  // Convert HTTP URL to WebSocket URL
  Some(if http_url.starts_with("https://") {
    format!("wss://{}/ws", &http_url["https://".len()..])
  } else if http_url.starts_with("http://") {
    format!("ws://{}/ws", &http_url["http://".len()..])
  } else if http_url.starts_with("ws://") || http_url.starts_with("wss://") {
    http_url
  } else {
    format!("ws://{}/ws", http_url)
  })
}

#[cfg(target_arch = "wasm32")]
fn current_page_url() -> Option<String> { web_sys::window().and_then(|w| w.location().href().ok()) }
