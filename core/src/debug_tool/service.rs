//! Shared service functions for HTTP handlers.
//!
//! These functions contain the core logic for HTTP endpoints.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]
#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

use std::sync::Arc;

use ribir_painter::PixelImage;
use serde_json::Value;
use tokio::sync::oneshot;

use super::{
  overlays::get_overlays,
  runtime::DebugServerState,
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

/// Platform-agnostic timeout helper.
#[cfg(not(target_arch = "wasm32"))]
async fn wait_with_timeout<T>(
  duration: std::time::Duration, future: impl std::future::Future<Output = T>,
) -> Option<T> {
  tokio::time::timeout(duration, future).await.ok()
}

/// Platform-agnostic timeout helper.
#[cfg(target_arch = "wasm32")]
async fn wait_with_timeout<T>(
  duration: std::time::Duration, future: impl std::future::Future<Output = T>,
) -> Option<T> {
  use futures::future::{Either, select};
  match select(Box::pin(future), Box::pin(gloo_timers::future::sleep(duration))).await {
    Either::Left((result, _)) => Some(result),
    Either::Right((_, _)) => None,
  }
}

/// Capture a screenshot, always requesting a fresh frame.
pub async fn capture_screenshot_svc(state: &DebugServerState) -> ServiceResult<Arc<PixelImage>> {
  // Clone and mark current value as seen BEFORE requesting redraw.
  let mut rx = state.last_frame_rx.clone();
  let _ = rx.borrow_and_update();

  // Request a redraw to get the latest frame.
  let _ = state
    .command_tx
    .send(DebugCommand::RequestRedraw { window_id: None })
    .await;

  // Wait with timeout (2s) for a new frame using cross-platform timer.
  let timeout_duration = std::time::Duration::from_secs(2);

  let frame_future = async {
    loop {
      if rx.changed().await.is_err() {
        return None;
      }
      if let Some(img) = rx.borrow_and_update().clone() {
        return Some(img);
      }
    }
  };

  let result = wait_with_timeout(timeout_duration, frame_future).await;

  match result.flatten() {
    Some(img) => Ok(img),
    None => {
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
#[allow(dead_code)]
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
