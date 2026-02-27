//! Debug MCP Server Module
//!
//! This module provides an HTTP-based debugging interface for inspecting
//! the widget tree, layout information, and visual debugging overlays.
//! Also implements the Model Context Protocol (MCP) for AI assistant
//! integration.
//!
//! Enabled via the `debug` feature flag.

mod helpers;
mod key_mapping;
mod mcp;
mod overlays;
mod port_registry;
mod server;
mod service;
mod types;

use std::sync::{Arc, OnceLock};

pub(crate) use overlays::paint_debug_overlays;
pub use overlays::{clear_overlays, set_overlay_hex};
use ribir_painter::PixelImage;
pub use server::start_debug_server;
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
