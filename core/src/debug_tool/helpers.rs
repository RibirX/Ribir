//! Helper functions for the debug MCP server.

use ribir_geom::{Point, Rect};
use ribir_painter::Color;
use serde_json::{Value, json};

use super::types::*;
use crate::{prelude::WidgetId, widget_tree::WidgetTree};

/// Build the layout tree recursively from WidgetTree.
///
/// Returns a JSON object with at least `{ "name": ... }`.
/// If `options.id`, includes `id`.
/// If node has children, includes `children`.
pub(crate) fn build_layout_tree_json(
  root: WidgetId, tree: &WidgetTree, options: InspectOptions,
) -> Value {
  let mut obj = build_layout_info_json(root, tree, options)
    .unwrap_or_else(|| json!({ "name": "Unknown", "error": "Widget not found" }));

  let children: Vec<Value> = root
    .children(tree)
    .map(|child| build_layout_tree_json(child, tree, options))
    .collect();

  if !children.is_empty()
    && let Some(obj_map) = obj.as_object_mut()
  {
    obj_map.insert("children".to_string(), Value::Array(children));
  }

  obj
}

/// Build detailed layout info for a specific widget.
///
/// Always returns a JSON object with at least `{ "name": ... }`.
/// Optional fields depend on `options`.
pub(crate) fn build_layout_info_json(
  id: WidgetId, tree: &WidgetTree, options: InspectOptions,
) -> Option<Value> {
  let render = id.get(tree)?;

  let mut obj = serde_json::Map::new();
  obj.insert("name".to_string(), Value::String(render.as_render().debug_name().into_owned()));

  if options.id {
    obj.insert("id".to_string(), serde_json::to_value(id).unwrap_or(Value::Null));
  }

  if options.props {
    obj.insert("properties".to_string(), render.as_render().debug_properties());
  }

  if options.layout
    && let Some(layout) = tree.store.layout_info(id)
  {
    let mut layout_obj = serde_json::Map::new();
    let pos = layout.pos;
    let clamp = &layout.clamp;

    layout_obj.insert("pos".to_string(), json!({"x": pos.x, "y": pos.y}));

    if options.global_pos {
      let global_pos = tree.map_to_global(Point::zero(), id);
      layout_obj.insert("global_pos".to_string(), json!({"x": global_pos.x, "y": global_pos.y}));
    }

    if let Some(size) = layout.size {
      layout_obj.insert("size".to_string(), json!({"width": size.width, "height": size.height}));
    }

    if options.clamp {
      layout_obj.insert(
        "constraints".to_string(),
        json!({
          "min": {"width": clamp.min.width, "height": clamp.min.height},
          "max": {"width": clamp.max.width, "height": clamp.max.height},
        }),
      );
    }
    obj.insert("layout".to_string(), Value::Object(layout_obj));
  }

  Some(Value::Object(obj))
}

/// Parse color from hex string (e.g., "#FF000080" or "#FF0000").
pub(crate) fn parse_hex_color(hex: &str) -> Option<Color> {
  let hex = hex.trim_start_matches('#');
  if hex.len() == 8 {
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
    Some(Color::from_u32((r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | a as u32))
  } else if hex.len() == 6 {
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_u32((r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | 0xFF))
  } else {
    None
  }
}

/// Get an overlay rect in global coordinates for a widget.
///
/// Uses the widget's layout box (pos+size) rather than visual bounds.
pub(crate) fn get_widget_global_overlay_rect(id: WidgetId, tree: &WidgetTree) -> Option<Rect> {
  let layout = tree.store.layout_info(id)?;
  let global_pos = match id.parent(tree) {
    Some(parent) => tree.map_to_global(layout.pos, parent),
    None => layout.pos,
  };

  let size = layout.size?;
  Some(Rect::new(global_pos, size))
}

pub(crate) fn parse_inspect_options(options: Option<&str>) -> InspectOptions {
  let mut out = InspectOptions::default();
  let Some(options) = options else {
    return out;
  };

  for token in options.split(',') {
    let token = token.trim().to_ascii_lowercase();
    match token.as_str() {
      "" => {}
      "all" => {
        out.id = true;
        out.layout = true;
        out.global_pos = true;
        out.clamp = true;
        out.props = true;
      }
      "id" => out.id = true,
      "layout" => out.layout = true,
      "global_pos" => {
        out.layout = true;
        out.global_pos = true;
      }
      "clamp" => {
        out.layout = true;
        out.clamp = true;
      }
      "no_global_pos" => out.global_pos = false,
      "no_clamp" => out.clamp = false,
      "no_props" => out.props = false,
      "props" => out.props = true,
      _ => {}
    }
  }

  out
}
