//! Data types for the debug MCP server.

use ribir_geom::Rect;
use serde::Deserialize;
use serde_json::Value;

use crate::{widget_tree::WidgetId, window::WindowId};

/// Controls which fields are collected and returned by the layout endpoints.
///
/// Default is minimal (only `name`, plus `children` in tree).
#[derive(Debug, Clone, Copy, Default)]
pub struct InspectOptions {
  pub id: bool,
  pub layout: bool,
  pub global_pos: bool,
  pub clamp: bool,
  pub props: bool,
}

/// Request body for POST /overlay.
#[derive(Debug, Clone, Deserialize)]
pub struct OverlayRequest {
  pub window_id: Option<WindowId>,
  pub id: String,
  /// Color in hex format with alpha, e.g., "#FF000080"
  pub color: String,
}

#[derive(serde::Serialize)]
pub struct WindowInfo {
  pub id: WindowId,
  pub title: String,
  pub width: f32,
  pub height: f32,
}

/// Command sent from the HTTP server to the main UI thread.
pub enum DebugCommand {
  InspectWidgetTree {
    window_id: Option<WindowId>,
    options: InspectOptions,
    reply: tokio::sync::oneshot::Sender<Value>,
  },
  InspectWidget {
    window_id: Option<WindowId>,
    id: String,
    options: InspectOptions,
    reply: tokio::sync::oneshot::Sender<Option<Value>>,
  },
  /// Query global rects for a list of widgets.
  ///
  /// The returned vector matches the input order.
  GetOverlayRects {
    window_id: Option<WindowId>,
    ids: Vec<WidgetId>,
    reply: tokio::sync::oneshot::Sender<Vec<Option<Rect>>>,
  },
  AddOverlay {
    window_id: Option<WindowId>,
    id: String,
    color: String,
    reply: tokio::sync::oneshot::Sender<bool>,
  },
  RemoveOverlay {
    window_id: Option<WindowId>,
    id: String,
    reply: tokio::sync::oneshot::Sender<bool>,
  },
  ClearOverlays {
    window_id: Option<WindowId>,
  },
  /// request a redraw of the window.
  RequestRedraw {
    window_id: Option<WindowId>,
  },
  /// Get list of available windows.
  GetWindows {
    reply: tokio::sync::oneshot::Sender<Vec<WindowInfo>>,
  },
}
