//! Shared service functions for HTTP and MCP handlers.
//!
//! These functions contain the core logic that both HTTP endpoints and MCP
//! tools share.

use std::sync::Arc;

use ribir_painter::PixelImage;
use serde_json::Value;
use tokio::sync::oneshot;

use super::{
  helpers,
  overlays::get_overlays,
  server::DebugServerState,
  types::{DebugCommand, InjectEventsResult, InjectedUiEvent, InspectOptions, WindowInfo},
};
use crate::window::WindowId;

/// Error type for service operations.
#[derive(Debug)]
pub enum ServiceError {
  /// The operation timed out.
  Timeout,
  /// The requested resource was not found.
  NotFound,
  /// Internal error occurred.
  Internal(String),
}

impl std::fmt::Display for ServiceError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ServiceError::Timeout => write!(f, "Operation timed out"),
      ServiceError::NotFound => write!(f, "Resource not found"),
      ServiceError::Internal(msg) => write!(f, "Internal error: {}", msg),
    }
  }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

/// Capture a screenshot, always requesting a fresh frame.
pub async fn capture_screenshot_svc(state: &DebugServerState) -> ServiceResult<Arc<PixelImage>> {
  // Clone and mark current value as seen BEFORE requesting redraw.
  // This ensures we wait for a genuinely NEW frame, not an old pending one.
  let mut rx = state.last_frame_rx.clone();
  let _ = rx.borrow_and_update(); // Mark current value as seen

  // Request a redraw to get the latest frame (with any overlays).
  let _ = state
    .command_tx
    .send(DebugCommand::RequestRedraw { window_id: None })
    .await;

  // Wait with timeout (2s) for a new frame
  let result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
    loop {
      if rx.changed().await.is_err() {
        return None;
      }
      if let Some(img) = rx.borrow_and_update().clone() {
        return Some(img);
      }
    }
  })
  .await;

  match result {
    Ok(Some(img)) => Ok(img),
    _ => {
      // Fallback: if timeout, try returning cached frame if available
      if let Some(img) = state.last_frame_rx.borrow().clone() {
        Ok(img)
      } else {
        Err(ServiceError::Timeout)
      }
    }
  }
}

/// Get the widget tree.
pub async fn inspect_tree_svc(
  state: &DebugServerState, window_id: Option<WindowId>, options: InspectOptions,
) -> ServiceResult<Value> {
  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .command_tx
    .send(DebugCommand::InspectWidgetTree { window_id, options, reply: reply_tx })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;

  reply_rx
    .await
    .map_err(|_| ServiceError::Internal("Failed to receive response".into()))
}

/// Get details for a specific widget.
pub async fn inspect_widget_svc(
  state: &DebugServerState, window_id: Option<WindowId>, widget_id: String, options: InspectOptions,
) -> ServiceResult<Value> {
  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .command_tx
    .send(DebugCommand::InspectWidget { window_id, id: widget_id, options, reply: reply_tx })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;

  reply_rx
    .await
    .map_err(|_| ServiceError::Internal("Failed to receive response".into()))?
    .ok_or(ServiceError::NotFound)
}

/// Get all windows.
pub async fn get_windows_svc(state: &DebugServerState) -> ServiceResult<Vec<WindowInfo>> {
  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .command_tx
    .send(DebugCommand::GetWindows { reply: reply_tx })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;

  reply_rx
    .await
    .map_err(|_| ServiceError::Internal("Failed to receive response".into()))
}

/// Get overlays for a window.
pub fn get_overlays_svc(window_id: WindowId) -> Vec<(crate::prelude::WidgetId, String)> {
  get_overlays(window_id)
}

/// Add an overlay.
pub async fn add_overlay_svc(
  state: &DebugServerState, window_id: Option<WindowId>, widget_id: String, color: String,
) -> ServiceResult<()> {
  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .command_tx
    .send(DebugCommand::AddOverlay { window_id, id: widget_id, color, reply: reply_tx })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;

  match reply_rx.await {
    Ok(true) => Ok(()),
    Ok(false) => Err(ServiceError::NotFound),
    Err(_) => Err(ServiceError::Internal("Failed to receive response".into())),
  }
}

/// Remove a specific overlay.
pub async fn remove_overlay_svc(
  state: &DebugServerState, window_id: Option<WindowId>, widget_id: String,
) -> ServiceResult<()> {
  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .command_tx
    .send(DebugCommand::RemoveOverlay { window_id, id: widget_id, reply: reply_tx })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;

  match reply_rx.await {
    Ok(true) => Ok(()),
    Ok(false) => Err(ServiceError::NotFound),
    Err(_) => Err(ServiceError::Internal("Failed to receive response".into())),
  }
}

/// Clear all overlays.
pub async fn clear_overlays_svc(
  state: &DebugServerState, window_id: Option<WindowId>,
) -> ServiceResult<()> {
  state
    .command_tx
    .send(DebugCommand::ClearOverlays { window_id })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;
  Ok(())
}

/// Inject serialized input events into the shared UI event loop path.
pub async fn inject_events_svc(
  state: &DebugServerState, window_id: Option<WindowId>, events: Vec<InjectedUiEvent>,
) -> ServiceResult<InjectEventsResult> {
  if events.is_empty() {
    return Err(ServiceError::Internal("events must not be empty".into()));
  }

  let (reply_tx, reply_rx) = oneshot::channel();
  state
    .command_tx
    .send(DebugCommand::InjectEvents { window_id, events, reply: reply_tx })
    .await
    .map_err(|_| ServiceError::Internal("Failed to send command".into()))?;

  match reply_rx.await {
    Ok(Ok(result)) => Ok(result),
    Ok(Err(msg)) => Err(ServiceError::Internal(msg)),
    Err(_) => Err(ServiceError::Internal("Failed to receive response".into())),
  }
}

/// Parse options string into InspectOptions.
pub fn parse_options(options_str: Option<&str>) -> InspectOptions {
  helpers::parse_inspect_options(options_str)
}
