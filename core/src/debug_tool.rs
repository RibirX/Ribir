//! Debug Bridge Client Module
//!
//! This module provides a WebSocket-based debugging interface that connects
//! to the CLI bridge server. Both Native and WASM targets use the same client
//! code, simplifying the architecture.
//!
//! Enabled via the `debug` feature flag.
//!
//! ## Architecture
//!
//! ```text
//! App (Native/WASM) --WebSocket--> CLI Bridge Server --HTTP--> Debug Tools/UI
//! ```
//!
//! The app acts as a WebSocket client, connecting to the CLI bridge server.
//! This eliminates duplicate HTTP server code and unifies Native/WASM
//! implementations.
//!
//! ## Module Organization
//!
//! - `bridge_client.rs`: Universal WebSocket client for both Native and WASM
//! - `service.rs`: Shared service logic (platform-independent)
//! - `runtime.rs`: Debug runtime state management
//! - `types.rs`: Shared type definitions
//! - `helpers.rs`: Helper functions for widget inspection
//! - `key_mapping.rs`: Keyboard event mapping utilities
//! - `overlays.rs`: Visual overlay management

mod bridge_client;
mod helpers;
mod key_mapping;
mod overlays;
mod runtime;
mod service;
mod types;

use std::sync::{Arc, OnceLock};

pub use bridge_client::start_debug_client;
pub(crate) use helpers::{OriginWidgetName, resolve_debug_name};
pub(crate) use overlays::paint_debug_overlays;
pub use overlays::{clear_overlays, set_overlay_hex};
pub use runtime::{is_macro_recording, record_ui_event};

#[cfg(not(target_arch = "wasm32"))]
pub fn now_unix_ms() -> u64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as u64
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn now_unix_ms() -> u64 { js_sys::Date::now() as u64 }

use ribir_painter::PixelImage;
use tokio::sync::mpsc;
pub use types::*;

use crate::window::WindowId;

#[derive(Clone, Debug)]
pub struct FramePacket {
  pub wnd_id: WindowId,
  pub ts_unix_ms: u64,
  pub seq: u64,
  pub image: Arc<PixelImage>,
}

/// Global channel sender for transmitting frames to the debug server.
pub static FRAME_TX: OnceLock<mpsc::UnboundedSender<FramePacket>> = OnceLock::new();
