use std::{
  collections::HashMap,
  sync::{OnceLock, RwLock},
};

use ribir_geom::Transform;
use ribir_painter::Painter;

use super::helpers::{get_widget_global_overlay_rect, parse_hex_color};
use crate::{
  widget_tree::{WidgetId, WidgetTree},
  window::WindowId,
};

static OVERLAYS: OnceLock<RwLock<HashMap<WindowId, HashMap<WidgetId, String>>>> = OnceLock::new();

fn overlay_store() -> &'static RwLock<HashMap<WindowId, HashMap<WidgetId, String>>> {
  OVERLAYS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn set_overlay_hex(win_id: WindowId, widget_id: WidgetId, color: &str) -> Option<()> {
  // Validate color format
  let _ = parse_hex_color(color)?;
  overlay_store()
    .write()
    .ok()?
    .entry(win_id)
    .or_default()
    .insert(widget_id, color.to_string());
  Some(())
}

pub fn clear_overlays(win_id: Option<WindowId>) {
  if let Ok(mut guard) = overlay_store().write() {
    if let Some(win_id) = win_id {
      guard.remove(&win_id);
    } else {
      guard.clear();
    }
  }
}

pub fn remove_overlay(win_id: WindowId, widget_id: WidgetId) -> Option<()> {
  overlay_store()
    .write()
    .ok()?
    .get_mut(&win_id)?
    .remove(&widget_id);
  Some(())
}

pub fn get_overlays(win_id: WindowId) -> Vec<(WidgetId, String)> {
  let lock = overlay_store().read();
  match lock {
    Ok(g) => g
      .get(&win_id)
      .map(|m| m.iter().map(|(id, c)| (*id, c.clone())).collect())
      .unwrap_or_default(),
    Err(e) => {
      eprintln!("[Ribir Debug] Overlay lock poisoned: {:?}", e);
      // Recover data from poisoned lock
      e.into_inner()
        .get(&win_id)
        .map(|m| m.iter().map(|(id, c)| (*id, c.clone())).collect())
        .unwrap_or_default()
    }
  }
}

pub(crate) fn paint_debug_overlays(win_id: WindowId, tree: &WidgetTree, painter: &mut Painter) {
  let overlays = get_overlays(win_id);

  if overlays.is_empty() {
    return;
  }

  painter.save();
  painter.set_transform(Transform::identity());

  for (id, color_str) in overlays {
    let Some(color) = parse_hex_color(&color_str) else { continue };
    let Some(r) = get_widget_global_overlay_rect(id, tree) else {
      continue;
    };
    if r.size.width <= 0.0 || r.size.height <= 0.0 {
      continue;
    }

    painter.set_fill_brush(color);
    painter.rect(&r, true);
    painter.fill();
  }

  painter.restore();
}
