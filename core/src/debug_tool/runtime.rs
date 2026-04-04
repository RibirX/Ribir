//! Debug runtime state management and command handling.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]
#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

use std::{
  collections::VecDeque,
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
  },
};

use ribir_algo::Rc;
use ribir_painter::PixelImage;
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, watch};
use winit::event::ElementState;

use super::{
  FRAME_TX, FramePacket, clear_overlays as clear_global_overlays,
  helpers::*,
  key_mapping::{
    derive_physical_key, infer_receive_chars_from_key, keyboard_key_error,
    keyboard_physical_key_error, parse_key_code, parse_virtual_key,
  },
  now_unix_ms,
  overlays::remove_overlay,
  set_overlay_hex,
  types::*,
};

/// Internal recorded event with timing metadata (not exposed in API).
#[derive(Debug, Clone)]
pub(crate) struct RecordedEvent {
  event: InjectedUiEvent,
  ts_unix_ms: u64,
}
use crate::{
  context::AppCtx,
  events::{KeyLocation, ModifiersState, MouseButtons, PhysicalKey, RibirDeviceId, VirtualKey},
  prelude::*,
  window::{UiEvent, WindowId},
};

/// Platform-agnostic timeout helper.
#[cfg(not(target_arch = "wasm32"))]
async fn timeout<T>(
  duration: std::time::Duration, future: impl std::future::Future<Output = T>,
) -> Option<T> {
  tokio::time::timeout(duration, future).await.ok()
}

/// Platform-agnostic timeout helper.
#[cfg(target_arch = "wasm32")]
async fn timeout<T>(
  duration: std::time::Duration, future: impl std::future::Future<Output = T>,
) -> Option<T> {
  use futures::future::{Either, select};
  match select(Box::pin(future), Box::pin(gloo_timers::future::sleep(duration))).await {
    Either::Left((result, _)) => Some(result),
    Either::Right((_, _)) => None,
  }
}

/// Global reference to the active recording session for non-async access.
static MACRO_RECORD_SESSION: Mutex<Option<Arc<EventMacroRecordingSession>>> = Mutex::new(None);

/// Record a UiEvent to the macro recorder.
/// Called from event_loop.rs::on_platform_event to capture all user
/// interactions.
pub fn record_ui_event(event: &crate::window::UiEvent) {
  // Fast-path: skip lock acquisition when not recording
  if !is_macro_recording() {
    return;
  }
  let session = {
    let guard = MACRO_RECORD_SESSION
      .lock()
      .unwrap_or_else(|e| e.into_inner());
    guard.clone()
  };
  let Some(session) = session else { return };

  use super::types::{InjectElementState, InjectKeyLocation, InjectMouseButton};
  use crate::events::MouseButtons;

  let injected = match event {
    UiEvent::CursorMoved { pos, .. } => InjectedUiEvent::CursorMoved { x: pos.x, y: pos.y },
    UiEvent::CursorLeft { .. } => InjectedUiEvent::CursorLeft,
    UiEvent::MouseWheel { delta_x, delta_y, .. } => {
      InjectedUiEvent::MouseWheel { delta_x: *delta_x, delta_y: *delta_y }
    }
    UiEvent::MouseInput { button, state, .. } => InjectedUiEvent::MouseInput {
      button: match *button {
        MouseButtons::PRIMARY => InjectMouseButton::Primary,
        MouseButtons::SECONDARY => InjectMouseButton::Secondary,
        MouseButtons::AUXILIARY => InjectMouseButton::Auxiliary,
        MouseButtons::FOURTH => InjectMouseButton::Fourth,
        MouseButtons::FIFTH => InjectMouseButton::Fifth,
        _ => InjectMouseButton::Primary,
      },
      state: match state {
        ElementState::Pressed => InjectElementState::Pressed,
        ElementState::Released => InjectElementState::Released,
      },
    },
    UiEvent::KeyBoard { key, state, .. } => InjectedUiEvent::RawKeyboardInput {
      key: format!("{:?}", key),
      physical_key: None,
      state: match state {
        ElementState::Pressed => InjectElementState::Pressed,
        ElementState::Released => InjectElementState::Released,
      },
      is_repeat: false,
      location: InjectKeyLocation::Standard,
      chars: None,
    },
    UiEvent::ReceiveChars { chars, .. } => InjectedUiEvent::Chars { chars: chars.to_string() },
    _ => return,
  };

  let ts = now_unix_ms();
  let mut events = session
    .events
    .lock()
    .unwrap_or_else(|e| e.into_inner());
  events.push(RecordedEvent { event: injected, ts_unix_ms: ts });
}

/// Check if macro recording is active.
pub fn is_macro_recording() -> bool {
  MACRO_RECORD_SESSION
    .lock()
    .unwrap_or_else(|e| e.into_inner())
    .is_some()
}

#[allow(dead_code)]
static CAPTURE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[allow(dead_code)]
static MACRO_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Global monotonic sequence for log entries (used for SSE cursor dedup).
static LOG_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct LogRingItem {
  seq: u64,
  ts_unix_ms: u64,
  line: Arc<str>,
}

/// Active event macro recording session.
enum MacroRecordingMode {
  Async,
  Timed {
    start_reply: Mutex<Option<tokio::sync::oneshot::Sender<Result<StartMacroResult, String>>>>,
  },
}

pub(crate) struct EventMacroRecordingSession {
  pub(crate) id: String,
  #[allow(dead_code)]
  pub(crate) window_id: Option<WindowId>,
  pub(crate) started_at_ts_unix_ms: u64,
  pub(crate) events: Mutex<Vec<RecordedEvent>>,
  mode: MacroRecordingMode,
}

fn finish_macro_recording(
  session: &EventMacroRecordingSession, end_ts_unix_ms: u64,
) -> StopEventMacroRecordingResult {
  let started_at = session.started_at_ts_unix_ms;
  let recorded_events: Vec<RecordedEvent> = std::mem::take(
    &mut session
      .events
      .lock()
      .unwrap_or_else(|e| e.into_inner()),
  );

  // Sort by timestamp and calculate delays
  let mut sorted_events = recorded_events;
  sorted_events.sort_by_key(|e| e.ts_unix_ms);

  // Build replay-ready events with Delay insertions
  let mut last_ts = started_at;
  let mut replay_events: Vec<InjectedUiEvent> = Vec::new();

  for recorded in &sorted_events {
    let delay = recorded.ts_unix_ms.saturating_sub(last_ts);
    if delay > 0 {
      replay_events.push(InjectedUiEvent::Delay { ms: delay });
    }
    replay_events.push(recorded.event.clone());
    last_ts = recorded.ts_unix_ms;
  }

  StopEventMacroRecordingResult {
    recording_id: session.id.clone(),
    events: replay_events,
    duration_ms: end_ts_unix_ms.saturating_sub(started_at),
  }
}

fn complete_timed_macro_start(
  session: &EventMacroRecordingSession, result: &StopEventMacroRecordingResult,
) {
  let MacroRecordingMode::Timed { start_reply } = &session.mode else {
    return;
  };
  if let Some(reply) = start_reply
    .lock()
    .unwrap_or_else(|e| e.into_inner())
    .take()
  {
    let _ = reply.send(Ok(StartMacroResult::WithEvents(result.clone())));
  }
}

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CaptureFrameEntry {
  seq: u64,
  ts_unix_ms: u64,
  path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  png_base64: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CaptureOptions {
  include_logs: bool,
  include_images: bool,
  pre_ms: u64,
  post_ms: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CaptureLogsPayload {
  lines: Vec<String>,
  dropped_total: u64,
  since_ts_unix_ms: u64,
  until_ts_unix_ms: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CaptureBundle {
  pub(crate) capture_id: String,
  pub(crate) start_ts_unix_ms: u64,
  pub(crate) end_ts_unix_ms: u64,
  pub(crate) options: CaptureOptions,
  pub(crate) filter_at_start: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) logs: Option<CaptureLogsPayload>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) frames: Option<Vec<CaptureFrameEntry>>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct CaptureSession {
  id: String,
  dir: PathBuf,
  start_ts_unix_ms: u64,
  options: CaptureOptions,
  filter_at_start: Option<String>,
  frames: Vec<CaptureFrameEntry>,
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
  pub(crate) recording: bool,
  pub(crate) log_sink_connected: bool,
  pub(crate) filter_reload_installed: bool,
  pub(crate) filter: Option<String>,
  pub(crate) dropped_total: u64,
  pub(crate) ring_len: usize,
  pub(crate) capture_root: String,
  pub(crate) active_capture: Option<StatusCaptureInfo>,
  /// Active macro recording ID, if any.
  #[serde(default)]
  pub(crate) active_macro_recording: Option<String>,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(serde::Deserialize)]
pub(crate) struct CaptureStartRequest {
  pub(crate) include: Vec<String>,
  #[serde(default)]
  pub(crate) pre_ms: Option<u64>,
  #[serde(default)]
  pub(crate) post_ms: Option<u64>,
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
}

#[derive(Debug)]
pub struct LogRing {
  items: VecDeque<LogRingItem>,
  max_items: usize,
  max_age_ms: u64,
}

impl LogRing {
  pub(crate) fn new(max_items: usize, max_age_ms: u64) -> Self {
    Self { items: VecDeque::new(), max_items, max_age_ms }
  }

  fn push(&mut self, item: LogRingItem) {
    self.items.push_back(item);

    while self.items.len() > self.max_items {
      self.items.pop_front();
    }

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

  pub fn len(&self) -> usize { self.items.len() }

  /// Returns the timestamp of the most recent log entry, if any.
  #[allow(dead_code)]
  pub fn max_ts(&self) -> Option<u64> { self.items.back().map(|i| i.ts_unix_ms) }

  /// Returns the sequence number of the most recent log entry, if any.
  #[allow(dead_code)]
  pub fn max_seq(&self) -> Option<u64> { self.items.back().map(|i| i.seq) }

  pub fn query_lines(
    &self, since_ts: Option<u64>, until_ts: Option<u64>, limit: Option<usize>,
  ) -> Vec<Arc<str>> {
    let mut out = Vec::new();

    let start_idx = if let Some(since) = since_ts {
      self
        .items
        .iter()
        .position(|item| item.ts_unix_ms >= since)
        .unwrap_or(self.items.len())
    } else {
      0
    };

    for item in self.items.iter().skip(start_idx) {
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

  /// Query lines starting from a specific sequence number (for SSE streaming).
  pub fn query_lines_from_seq(&self, from_seq: u64, limit: Option<usize>) -> Vec<(u64, Arc<str>)> {
    let mut out = Vec::new();

    let start_idx = self
      .items
      .iter()
      .position(|item| item.seq >= from_seq)
      .unwrap_or(self.items.len());

    for item in self.items.iter().skip(start_idx) {
      out.push((item.seq, item.line.clone()));
      if limit.is_some_and(|l| out.len() >= l) {
        break;
      }
    }
    out
  }
}

/// Shared state for debug transports.
pub struct DebugServerState {
  pub command_tx: mpsc::Sender<DebugCommand>,
  pub recording: AtomicBool,
  pub last_frame_rx: watch::Receiver<Option<Arc<PixelImage>>>,
  pub last_frame_tx: watch::Sender<Option<Arc<PixelImage>>>,
  pub log_ring: tokio::sync::Mutex<LogRing>,
  /// Broadcasts (seq, ring_len) tuples when new logs arrive.
  pub log_broadcast: broadcast::Sender<(u64, usize)>,
  pub capture_root: PathBuf,
  pub active_capture: tokio::sync::Mutex<Option<CaptureSession>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureError {
  Conflict,
  NotFound,
  Timeout,
  Internal,
}

pub(crate) fn start_debug_runtime() -> Arc<DebugServerState> {
  crate::logging::init_debug_tracing("info");

  let (cmd_tx, mut cmd_rx) = mpsc::channel::<DebugCommand>(32);
  let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<FramePacket>();
  let (last_frame_tx, last_frame_rx) = watch::channel::<Option<Arc<PixelImage>>>(None);

  let (log_tx, mut log_rx) = mpsc::unbounded_channel::<crate::logging::LogLine>();
  crate::logging::install_debug_log_sender(log_tx);

  let (log_broadcast, _) = broadcast::channel::<(u64, usize)>(1024);

  let capture_root = std::env::var("RIBIR_CAPTURE_DIR")
    .ok()
    .filter(|s| !s.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("captures"));

  let _ = FRAME_TX.set(frame_tx);

  let state = Arc::new(DebugServerState {
    command_tx: cmd_tx.clone(),
    recording: AtomicBool::new(false),
    last_frame_rx,
    last_frame_tx,
    log_ring: tokio::sync::Mutex::new(LogRing::new(50_000, 60_000)),
    log_broadcast,
    capture_root,
    active_capture: tokio::sync::Mutex::new(None),
  });

  let state_clone = state.clone();
  AppCtx::spawn(async move {
    while let Some(pkt) = frame_rx.recv().await {
      let _ = state_clone
        .last_frame_tx
        .send_replace(Some(pkt.image.clone()));

      #[cfg(not(target_arch = "wasm32"))]
      if state_clone.recording.load(Ordering::Relaxed)
        && state_clone.active_capture.lock().await.is_none()
      {
        let filename = format!("frame_{}_{}.webp", pkt.ts_unix_ms, pkt.seq);

        // Spawn blocking IO task to save image
        let img_clone = pkt.image.clone();
        tokio::task::spawn_blocking(move || {
          let mut data = Vec::new();
          if img_clone.write_as_webp(&mut data).is_ok() {
            let _ = std::fs::write(filename, data);
          }
        });
      }

      // Handle capture session: save frame and update session state
      let mut guard = state_clone.active_capture.lock().await;
      if let Some(session) = guard.as_mut()
        && session.options.include_images
      {
        let ts_unix_ms = pkt.ts_unix_ms;
        let seq = pkt.seq;
        let rel_path = format!("frames/frame_{}_{}.webp", ts_unix_ms, seq);

        #[cfg(not(target_arch = "wasm32"))]
        {
          let abs_path = session.dir.join(&rel_path);
          session.frames.push(CaptureFrameEntry {
            seq,
            ts_unix_ms,
            path: rel_path,
            png_base64: None,
          });
          // Clone image before dropping the lock
          let img_clone = pkt.image.clone();

          // Drop the lock before spawning blocking task
          drop(guard);

          tokio::task::spawn_blocking(move || {
            let _ = std::fs::create_dir_all(
              abs_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
            );
            let mut data = Vec::new();
            if img_clone.write_as_webp(&mut data).is_ok() {
              let _ = std::fs::write(abs_path, data);
            }
          });
        }

        #[cfg(target_arch = "wasm32")]
        {
          use base64::{Engine as _, engine::general_purpose};

          let mut png_data = Vec::new();
          if pkt.image.write_as_webp(&mut png_data).is_ok() {
            session.frames.push(CaptureFrameEntry {
              seq,
              ts_unix_ms,
              path: rel_path,
              png_base64: Some(general_purpose::STANDARD.encode(png_data)),
            });
          }
        }
      }
    }
  });

  let log_state = state.clone();
  AppCtx::spawn_local(async move {
    while let Some(log_line) = log_rx.recv().await {
      let seq = LOG_SEQ.fetch_add(1, Ordering::Relaxed);

      let line_with_seq = if let Ok(mut json) = serde_json::from_str::<Value>(&log_line.line) {
        if let Some(obj) = json.as_object_mut() {
          obj.insert("seq".to_string(), Value::from(seq));
        }
        json.to_string()
      } else {
        log_line.line.to_string()
      };

      let item =
        LogRingItem { seq, ts_unix_ms: log_line.ts_unix_ms, line: Arc::from(line_with_seq) };
      {
        let mut ring = log_state.log_ring.lock().await;
        let ring_len = ring.len();
        ring.push(item);
        match log_state.log_broadcast.send((seq, ring_len)) {
          Ok(_) => {}
          Err(broadcast::error::SendError((seq, _))) => {
            tracing::debug!(seq, "log broadcast: no active receivers");
          }
        }
      }
    }
  });

  let state_for_return = state.clone();
  AppCtx::spawn_local(async move {
    while let Some(cmd) = cmd_rx.recv().await {
      handle_command(cmd, state.clone()).await;
    }
  });

  state_for_return
}

pub(crate) async fn build_status_response(state: &DebugServerState) -> StatusResponse {
  let ring_len = { state.log_ring.lock().await.len() };

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

  let active_macro_recording = {
    MACRO_RECORD_SESSION
      .lock()
      .unwrap_or_else(|e| e.into_inner())
      .as_ref()
      .map(|s| s.id.clone())
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
    active_macro_recording,
  }
}

async fn take_capture_session(
  state: &DebugServerState, payload: &CaptureStopRequest,
) -> Result<CaptureSession, CaptureError> {
  let mut guard = state.active_capture.lock().await;
  let Some(session) = guard.as_ref() else {
    return Err(CaptureError::NotFound);
  };
  if let Some(req_id) = payload.capture_id.as_ref()
    && req_id != &session.id
  {
    return Err(CaptureError::NotFound);
  }
  guard.take().ok_or(CaptureError::NotFound)
}

async fn build_capture_bundle(
  state: &DebugServerState, session: &CaptureSession, end_ts_unix_ms: u64,
) -> CaptureBundle {
  let logs_since = session
    .start_ts_unix_ms
    .saturating_sub(session.options.pre_ms);
  let logs_until = end_ts_unix_ms.saturating_add(session.options.post_ms);
  let logs = if session.options.include_logs {
    let ring = state.log_ring.lock().await;
    let lines = ring
      .query_lines(Some(logs_since), Some(logs_until), None)
      .into_iter()
      .map(|line| line.to_string())
      .collect();
    Some(CaptureLogsPayload {
      lines,
      dropped_total: crate::logging::dropped_logs_total(),
      since_ts_unix_ms: logs_since,
      until_ts_unix_ms: logs_until,
    })
  } else {
    None
  };

  CaptureBundle {
    capture_id: session.id.clone(),
    start_ts_unix_ms: session.start_ts_unix_ms,
    end_ts_unix_ms,
    options: session.options.clone(),
    filter_at_start: session.filter_at_start.clone(),
    logs,
    frames: session
      .options
      .include_images
      .then(|| session.frames.clone()),
  }
}

pub(crate) async fn capture_start_inner(
  state: Arc<DebugServerState>, include: Vec<String>, pre_ms: u64, post_ms: u64,
  output_dir: Option<String>,
) -> Result<CaptureStartResponse, CaptureError> {
  let include_logs = include.iter().any(|s| s == "logs");
  let include_images = include.iter().any(|s| s == "images");
  let options = CaptureOptions { include_logs, include_images, pre_ms, post_ms };

  let mut guard = state.active_capture.lock().await;
  if guard.is_some() {
    return Err(CaptureError::Conflict);
  }

  let start_ts_unix_ms = now_unix_ms();

  let seq = CAPTURE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  let capture_id = format!("cap_{}_{}", start_ts_unix_ms, seq);

  let root = output_dir
    .as_ref()
    .map(PathBuf::from)
    .unwrap_or_else(|| state.capture_root.clone());
  let capture_dir = absolutize_path(root.join(&capture_id));

  #[cfg(not(target_arch = "wasm32"))]
  {
    let capture_dir_clone = capture_dir.clone();
    tokio::task::spawn_blocking(move || {
      let _ = std::fs::create_dir_all(capture_dir_clone.join("frames"));
    })
    .await
    .map_err(|_| CaptureError::Internal)?;
  }

  *guard = Some(CaptureSession {
    id: capture_id.clone(),
    dir: capture_dir.clone(),
    start_ts_unix_ms,
    options,
    filter_at_start: crate::logging::current_filter_string(),
    frames: Vec::new(),
  });

  Ok(CaptureStartResponse { capture_id, capture_dir: capture_dir.to_string_lossy().to_string() })
}

pub(crate) async fn capture_stop_bundle_inner(
  state: Arc<DebugServerState>, payload: CaptureStopRequest,
) -> Result<CaptureBundle, CaptureError> {
  let end_ts_unix_ms = now_unix_ms();
  let session = take_capture_session(state.as_ref(), &payload).await?;
  Ok(build_capture_bundle(state.as_ref(), &session, end_ts_unix_ms).await)
}

pub(crate) async fn capture_one_shot_bundle_inner(
  state: Arc<DebugServerState>, payload: CaptureOneShotRequest,
) -> Result<CaptureBundle, CaptureError> {
  let settle_ms = payload.settle_ms.unwrap_or(150);
  let include_images = payload.include.iter().any(|s| s == "images");

  let _start = capture_start_inner(
    state.clone(),
    payload.include,
    payload.pre_ms.unwrap_or(2_000),
    payload.post_ms.unwrap_or(1_000),
    None,
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

    let mut rx = state.last_frame_rx.clone();
    let _ = rx.borrow_and_update();
    let _ = state
      .command_tx
      .send(DebugCommand::RequestRedraw { window_id: None })
      .await;

    let waited_new_frame = timeout(std::time::Duration::from_millis(1200), async {
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

    AppCtx::timer(std::time::Duration::from_millis(settle_ms)).await;
    AppCtx::timer(std::time::Duration::from_millis(80)).await;

    if !waited_new_frame {
      let _ =
        capture_stop_bundle_inner(state.clone(), CaptureStopRequest { capture_id: None }).await;
      return Err(CaptureError::Timeout);
    }
  }

  capture_stop_bundle_inner(state, CaptureStopRequest { capture_id: None }).await
}

pub(crate) fn resolve_widget_id(id_str: &str, tree: &WidgetTree) -> Option<WidgetId> {
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

  let name = id_str
    .strip_prefix("name:")
    .or_else(|| id_str.strip_prefix("debug:"))
    .unwrap_or(id_str)
    .trim();
  if name.is_empty() {
    return None;
  }

  resolve_widget_id_by_debug_name(tree, name)
}

fn resolve_widget_id_by_debug_name(tree: &WidgetTree, name: &str) -> Option<WidgetId> {
  let root = tree.root();
  let mut stack = vec![root];
  while let Some(node) = stack.pop() {
    if let Some(label) = node
      .query_ref::<OriginWidgetName>(tree)
      .map(|n| n.0.clone())
    {
      if &*label == name {
        return Some(node);
      }
    } else if let Some(render) = node.get(tree) {
      let dbg_name = render.as_render().debug_name();
      if dbg_name.as_ref() == name {
        return Some(node);
      }
    }
    stack.extend(node.children(tree));
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

pub(crate) fn absolutize_path(path: impl Into<PathBuf>) -> PathBuf {
  let path = path.into();
  if path.is_absolute() {
    return path;
  }
  std::env::current_dir()
    .map(|cwd| cwd.join(&path))
    .unwrap_or(path)
}

pub(crate) fn resolve_target_window(requested_id: Option<WindowId>) -> Option<Rc<Window>> {
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

async fn handle_command(cmd: DebugCommand, _state: Arc<DebugServerState>) {
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
        wnd
          .shell_wnd()
          .borrow()
          .request_draw(crate::window::RedrawDemand::Force);
      }
    }

    DebugCommand::InjectEvents { window_id, events, reply } => {
      let Some(target_wnd) = resolve_target_window(window_id) else {
        let _ = reply.send(Err("No active window found".into()));
        return;
      };

      let mut accepted = 0usize;

      for event in events {
        if let InjectedUiEvent::Delay { ms } = event {
          AppCtx::timer(std::time::Duration::from_millis(ms)).await;
          continue;
        }

        let ui_events = match injected_to_ui_events(&target_wnd, event) {
          Ok(events) => events,
          Err(msg) => {
            let _ = reply.send(Err(msg));
            return;
          }
        };

        for ui_event in ui_events {
          if AppCtx::send_ui_event(ui_event) {
            accepted += 1;
          } else {
            let _ = reply.send(Err("Failed to send UiEvent to event loop".into()));
            return;
          }
        }
      }

      let _ = reply.send(Ok(InjectEventsResult { accepted }));
    }

    DebugCommand::StartEventMacroRecording { window_id, duration_ms, reply } => {
      let guard = MACRO_RECORD_SESSION
        .lock()
        .unwrap_or_else(|e| e.into_inner());
      if guard.is_some() {
        let _ = reply.send(Err("Event macro recording already in progress. Stop it first.".into()));
        return;
      }
      drop(guard);

      let started_at = now_unix_ms();
      let seq = MACRO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      let recording_id = format!("macro_{}_{}", started_at, seq);

      if let Some(duration_ms) = duration_ms {
        // Synchronous mode: wait for duration, then return full result
        let session = Arc::new(EventMacroRecordingSession {
          id: recording_id.clone(),
          window_id,
          started_at_ts_unix_ms: started_at,
          events: Mutex::new(Vec::new()),
          mode: MacroRecordingMode::Timed { start_reply: Mutex::new(Some(reply)) },
        });
        let _ = MACRO_RECORD_SESSION
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .replace(session.clone());

        AppCtx::spawn(async move {
          AppCtx::timer(std::time::Duration::from_millis(duration_ms)).await;

          // Stop recording and build result
          let session = {
            MACRO_RECORD_SESSION
              .lock()
              .unwrap_or_else(|e| e.into_inner())
              .take()
          };
          let Some(session) = session else { return };
          let result = finish_macro_recording(session.as_ref(), now_unix_ms());
          complete_timed_macro_start(session.as_ref(), &result);
        });
      } else {
        // Async mode
        let session = Arc::new(EventMacroRecordingSession {
          id: recording_id.clone(),
          window_id,
          started_at_ts_unix_ms: started_at,
          events: Mutex::new(Vec::new()),
          mode: MacroRecordingMode::Async,
        });
        let _ = MACRO_RECORD_SESSION
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .replace(session);

        let _ = reply.send(Ok(StartMacroResult::Started(StartEventMacroRecordingResult {
          recording_id,
          started_at_ts_unix_ms: started_at,
        })));
      }
    }

    DebugCommand::StopEventMacroRecording { recording_id, reply } => {
      let session = {
        MACRO_RECORD_SESSION
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .take()
      };
      let Some(session) = session else {
        let _ = reply.send(Err("No active event macro recording session.".into()));
        return;
      };

      if let Some(ref req_id) = recording_id
        && req_id != &session.id
      {
        // Put it back if ID doesn't match
        let active_id = session.id.clone();
        MACRO_RECORD_SESSION
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .replace(session);
        let _ = reply.send(Err(format!(
          "Recording ID mismatch: requested {}, active {}",
          req_id.as_str(),
          active_id
        )));
        return;
      }

      let result = finish_macro_recording(session.as_ref(), now_unix_ms());
      complete_timed_macro_start(session.as_ref(), &result);
      let _ = reply.send(Ok(result));
    }
  }
}

fn resolve_injected_click_pos(
  wnd: &Window, x: Option<f32>, y: Option<f32>, id: Option<String>,
) -> Result<Option<ribir_types::Point>, String> {
  match (x, y) {
    (Some(px), Some(py)) => return Ok(Some(ribir_types::Point::new(px, py))),
    (Some(_), None) | (None, Some(_)) => {
      return Err("click/double_click requires both x and y when using coordinates".into());
    }
    (None, None) => {}
  }

  let Some(id_str) = id else {
    return Ok(None);
  };

  let tree = wnd.tree();
  let Some(widget_id) = resolve_widget_id(&id_str, tree) else {
    return Err(format!(
      "Widget not found for click target id '{}'. Supported formats: '3', '3:0', \
       '{{\"index1\":3,\"stamp\":0}}', or 'name:<debug_name>'.",
      id_str
    ));
  };

  let Some(layout) = tree.store.layout_info(widget_id) else {
    return Err(format!("Widget '{}' has no layout info", id_str));
  };
  let Some(size) = layout.size else {
    return Err(format!("Widget '{}' has no resolved size", id_str));
  };

  let global_pos = match widget_id.parent(tree) {
    Some(parent) => tree.map_to_global(layout.pos, parent),
    None => layout.pos,
  };

  Ok(Some(ribir_types::Point::new(global_pos.x + size.width / 2., global_pos.y + size.height / 2.)))
}

fn mouse_button(button: InjectMouseButton) -> MouseButtons {
  match button {
    InjectMouseButton::Primary => MouseButtons::PRIMARY,
    InjectMouseButton::Secondary => MouseButtons::SECONDARY,
    InjectMouseButton::Auxiliary => MouseButtons::AUXILIARY,
    InjectMouseButton::Fourth => MouseButtons::FOURTH,
    InjectMouseButton::Fifth => MouseButtons::FIFTH,
  }
}

fn key_location(location: InjectKeyLocation) -> KeyLocation {
  match location {
    InjectKeyLocation::Standard => KeyLocation::Standard,
    InjectKeyLocation::Left => KeyLocation::Left,
    InjectKeyLocation::Right => KeyLocation::Right,
    InjectKeyLocation::Numpad => KeyLocation::Numpad,
  }
}

fn resolve_chars_payload(chars: Option<String>, key: &str) -> Option<String> {
  chars
    .filter(|text| !text.is_empty())
    .or_else(|| infer_receive_chars_from_key(key))
}

fn build_keyboard_event(
  window_id: WindowId, key: VirtualKey, state: ElementState, physical_key: PhysicalKey,
  is_repeat: bool, location: KeyLocation,
) -> UiEvent {
  UiEvent::KeyBoard { wnd_id: window_id, key, state, physical_key, is_repeat, location }
}

fn build_keyboard_input_events(
  window_id: WindowId, key: String, chars: Option<String>,
) -> Result<Vec<UiEvent>, String> {
  let key_value = parse_virtual_key(&key).ok_or_else(|| keyboard_key_error(&key))?;
  let physical_key = derive_physical_key(&key).ok_or_else(|| {
    format!(
      "Cannot derive physical_key from key '{}'. Use `raw_keyboard_input` with explicit \
       `physical_key` (e.g. KeyA, Digit1, Enter).",
      key
    )
  })?;

  let mut events = Vec::with_capacity(3);
  events.push(build_keyboard_event(
    window_id,
    key_value.clone(),
    ElementState::Pressed,
    physical_key,
    false,
    KeyLocation::Standard,
  ));

  if let Some(text) = resolve_chars_payload(chars, &key) {
    events.push(UiEvent::ReceiveChars { wnd_id: window_id, chars: text.into() });
  }

  events.push(build_keyboard_event(
    window_id,
    key_value,
    ElementState::Released,
    physical_key,
    false,
    KeyLocation::Standard,
  ));

  Ok(events)
}

fn build_raw_keyboard_input_events(
  window_id: WindowId, key: String, physical_key: Option<String>, state: InjectElementState,
  is_repeat: bool, location: InjectKeyLocation, chars: Option<String>,
) -> Result<Vec<UiEvent>, String> {
  let key_value = parse_virtual_key(&key).ok_or_else(|| keyboard_key_error(&key))?;
  let physical_key = match physical_key {
    Some(value) => {
      let code = parse_key_code(&value).ok_or_else(|| keyboard_physical_key_error(&value))?;
      PhysicalKey::Code(code)
    }
    None => derive_physical_key(&key).ok_or_else(|| {
      format!(
        "Cannot derive physical_key from key '{}'. Provide `physical_key` with W3C code names \
         (e.g. KeyA, Digit1, Enter).",
        key
      )
    })?,
  };

  let event_state = ElementState::from(state.clone());
  let mut events = vec![build_keyboard_event(
    window_id,
    key_value,
    event_state,
    physical_key,
    is_repeat,
    key_location(location),
  )];

  if matches!(state, InjectElementState::Pressed)
    && let Some(text) = resolve_chars_payload(chars, &key)
  {
    events.push(UiEvent::ReceiveChars { wnd_id: window_id, chars: text.into() });
  }

  Ok(events)
}

fn injected_to_ui_events(wnd: &Window, event: InjectedUiEvent) -> Result<Vec<UiEvent>, String> {
  let window_id = wnd.id();
  let ui_events = match event {
    InjectedUiEvent::Delay { .. } => unreachable!("Delay should be handled before conversion"),
    InjectedUiEvent::CursorMoved { x, y } => {
      vec![UiEvent::CursorMoved { wnd_id: window_id, pos: ribir_types::Point::new(x, y) }]
    }
    InjectedUiEvent::CursorLeft => vec![UiEvent::CursorLeft { wnd_id: window_id }],
    InjectedUiEvent::MouseWheel { delta_x, delta_y } => {
      vec![UiEvent::MouseWheel { wnd_id: window_id, delta_x, delta_y }]
    }
    InjectedUiEvent::MouseInput { button, state } => vec![UiEvent::MouseInput {
      wnd_id: window_id,
      device_id: Box::new(RibirDeviceId::Dummy),
      button: mouse_button(button),
      state: ElementState::from(state),
    }],
    InjectedUiEvent::KeyboardInput { key, chars } => {
      build_keyboard_input_events(window_id, key, chars)?
    }
    InjectedUiEvent::RawKeyboardInput { key, physical_key, state, is_repeat, location, chars } => {
      build_raw_keyboard_input_events(
        window_id,
        key,
        physical_key,
        state,
        is_repeat,
        location,
        chars,
      )?
    }
    InjectedUiEvent::Click { button, id, x, y } => {
      let mut out = Vec::with_capacity(3);
      if let Some(pos) = resolve_injected_click_pos(wnd, x, y, id)? {
        out.push(UiEvent::CursorMoved { wnd_id: window_id, pos });
      }
      let mapped_button = mouse_button(button);
      out.push(UiEvent::MouseInput {
        wnd_id: window_id,
        device_id: Box::new(RibirDeviceId::Dummy),
        button: mapped_button,
        state: ElementState::Pressed,
      });
      out.push(UiEvent::MouseInput {
        wnd_id: window_id,
        device_id: Box::new(RibirDeviceId::Dummy),
        button: mapped_button,
        state: ElementState::Released,
      });
      out
    }
    InjectedUiEvent::DoubleClick { button, id, x, y } => {
      let mut out = Vec::with_capacity(5);
      if let Some(pos) = resolve_injected_click_pos(wnd, x, y, id)? {
        out.push(UiEvent::CursorMoved { wnd_id: window_id, pos });
      }
      let mapped_button = mouse_button(button);
      let make_input = |state| UiEvent::MouseInput {
        wnd_id: window_id,
        device_id: Box::new(RibirDeviceId::Dummy),
        button: mapped_button,
        state,
      };
      out.push(make_input(ElementState::Pressed));
      out.push(make_input(ElementState::Released));
      out.push(make_input(ElementState::Pressed));
      out.push(make_input(ElementState::Released));
      out
    }
    InjectedUiEvent::Chars { chars } => {
      vec![UiEvent::ReceiveChars { wnd_id: window_id, chars: chars.into() }]
    }
    InjectedUiEvent::ModifiersChanged { shift, ctrl, alt, logo } => {
      let mut state = ModifiersState::empty();
      if shift {
        state |= ModifiersState::SHIFT;
      }
      if ctrl {
        state |= ModifiersState::CONTROL;
      }
      if alt {
        state |= ModifiersState::ALT;
      }
      if logo {
        state |= ModifiersState::SUPER;
      }
      vec![UiEvent::ModifiersChanged { wnd_id: window_id, state }]
    }
    InjectedUiEvent::RedrawRequest { force } => {
      vec![UiEvent::RedrawRequest {
        wnd_id: window_id,
        demand: crate::window::RedrawDemand::from_force(force),
      }]
    }
  };

  Ok(ui_events)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn finish_macro_recording_reports_wall_clock_duration() {
    let session = EventMacroRecordingSession {
      id: "macro_test".to_string(),
      window_id: None,
      started_at_ts_unix_ms: 1_000,
      events: Mutex::new(vec![
        RecordedEvent { event: InjectedUiEvent::CursorLeft, ts_unix_ms: 1_100 },
        RecordedEvent {
          event: InjectedUiEvent::Chars { chars: "hello".to_string() },
          ts_unix_ms: 1_300,
        },
      ]),
      mode: MacroRecordingMode::Async,
    };

    let result = finish_macro_recording(&session, 2_000);

    assert_eq!(result.duration_ms, 1_000);
    // Should have 4 events: Delay(100), CursorLeft, Delay(200), Chars
    assert_eq!(result.events.len(), 4);
    // First event is Delay(100ms)
    if let InjectedUiEvent::Delay { ms } = &result.events[0] {
      assert_eq!(*ms, 100);
    } else {
      panic!("Expected Delay event");
    }
    // Second event is CursorLeft
    assert!(matches!(&result.events[1], InjectedUiEvent::CursorLeft));
    // Third event is Delay(200ms)
    if let InjectedUiEvent::Delay { ms } = &result.events[2] {
      assert_eq!(*ms, 200);
    } else {
      panic!("Expected Delay event");
    }
    // Fourth event is Chars
    assert!(matches!(&result.events[3], InjectedUiEvent::Chars { .. }));
  }

  #[test]
  fn complete_timed_macro_start_only_resolves_once() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let session = EventMacroRecordingSession {
      id: "macro_test".to_string(),
      window_id: None,
      started_at_ts_unix_ms: 1_000,
      events: Mutex::new(Vec::new()),
      mode: MacroRecordingMode::Timed { start_reply: Mutex::new(Some(tx)) },
    };
    let result = StopEventMacroRecordingResult {
      recording_id: "macro_test".to_string(),
      events: Vec::new(),
      duration_ms: 2_000,
    };

    complete_timed_macro_start(&session, &result);
    complete_timed_macro_start(&session, &result);

    let received = rx
      .blocking_recv()
      .expect("timed start should resolve exactly once")
      .expect("timed start should resolve successfully");
    match received {
      StartMacroResult::WithEvents(resolved) => {
        assert_eq!(resolved.recording_id, "macro_test");
        assert_eq!(resolved.duration_ms, 2_000);
      }
      StartMacroResult::Started(_) => panic!("timed completion should return recorded events"),
    }
  }
}
