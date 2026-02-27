//! Data types for the debug MCP server.

use serde::{Deserialize, de::Error as DeError};
use serde_json::Value;
use winit::event::ElementState;

use crate::{events::RibirDeviceId, window::WindowId};

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

#[derive(Debug, Clone, Deserialize)]
pub struct InjectEventsRequest {
  pub window_id: Option<WindowId>,
  pub events: Vec<InjectedUiEvent>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InjectEventsResult {
  pub accepted: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InjectedUiEvent {
  CursorMoved {
    x: f32,
    y: f32,
  },
  CursorLeft,
  MouseWheel {
    delta_x: f32,
    delta_y: f32,
  },
  MouseInput {
    #[serde(default)]
    device_id: InjectDeviceId,
    button: InjectMouseButton,
    state: InjectElementState,
  },
  /// High-level keyboard event: key press/release with optional chars payload.
  KeyboardInput {
    key: String,
    #[serde(default)]
    chars: Option<String>,
  },
  /// Low-level keyboard event with full control over physical key metadata.
  RawKeyboardInput {
    key: String,
    #[serde(default)]
    physical_key: Option<String>,
    state: InjectElementState,
    #[serde(default)]
    is_repeat: bool,
    #[serde(default)]
    location: InjectKeyLocation,
    #[serde(default)]
    chars: Option<String>,
  },
  Click {
    #[serde(default)]
    device_id: InjectDeviceId,
    #[serde(default)]
    button: InjectMouseButton,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    x: Option<f32>,
    #[serde(default)]
    y: Option<f32>,
  },
  DoubleClick {
    #[serde(default)]
    device_id: InjectDeviceId,
    #[serde(default)]
    button: InjectMouseButton,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    x: Option<f32>,
    #[serde(default)]
    y: Option<f32>,
  },
  Chars {
    chars: String,
  },
  ModifiersChanged {
    #[serde(default)]
    shift: bool,
    #[serde(default)]
    ctrl: bool,
    #[serde(default)]
    alt: bool,
    #[serde(default)]
    logo: bool,
  },
  RedrawRequest {
    #[serde(default)]
    force: bool,
  },
}

#[derive(Debug, Clone, Copy, Default)]
pub enum InjectMouseButton {
  #[default]
  Primary,
  Secondary,
  Auxiliary,
  Fourth,
  Fifth,
}

#[derive(Debug, Clone)]
pub enum InjectElementState {
  Pressed,
  Released,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum InjectKeyLocation {
  #[default]
  Standard,
  Left,
  Right,
  Numpad,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum InjectDeviceId {
  #[default]
  Dummy,
  Custom(u64),
}

impl From<InjectDeviceId> for RibirDeviceId {
  fn from(value: InjectDeviceId) -> Self {
    match value {
      InjectDeviceId::Dummy => RibirDeviceId::Dummy,
      InjectDeviceId::Custom(id) => RibirDeviceId::Custom(id),
    }
  }
}

impl From<InjectElementState> for ElementState {
  fn from(value: InjectElementState) -> Self {
    match value {
      InjectElementState::Pressed => ElementState::Pressed,
      InjectElementState::Released => ElementState::Released,
    }
  }
}

fn normalize_token(value: &str) -> String {
  value
    .trim()
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .map(|c| c.to_ascii_lowercase())
    .collect()
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
  let a_chars: Vec<char> = a.chars().collect();
  let b_chars: Vec<char> = b.chars().collect();
  let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
  let mut cur = vec![0; b_chars.len() + 1];

  for (i, &ca) in a_chars.iter().enumerate() {
    cur[0] = i + 1;
    for (j, &cb) in b_chars.iter().enumerate() {
      let cost = usize::from(ca != cb);
      cur[j + 1] = (prev[j + 1] + 1)
        .min(cur[j] + 1)
        .min(prev[j] + cost);
    }
    std::mem::swap(&mut prev, &mut cur);
  }

  prev[b_chars.len()]
}

fn closest_matches<'a>(input: &str, candidates: &'a [&'a str], limit: usize) -> Vec<&'a str> {
  let normalized_input = normalize_token(input);
  let mut scored: Vec<(usize, bool, bool, &'a str)> = candidates
    .iter()
    .copied()
    .map(|candidate| {
      let normalized_candidate = normalize_token(candidate);
      (
        levenshtein_distance(&normalized_input, &normalized_candidate),
        normalized_candidate.starts_with(&normalized_input),
        normalized_candidate.contains(&normalized_input),
        candidate,
      )
    })
    .collect();

  scored.sort_by(|a, b| {
    a.0
      .cmp(&b.0)
      .then_with(|| b.1.cmp(&a.1))
      .then_with(|| b.2.cmp(&a.2))
  });
  scored
    .into_iter()
    .take(limit)
    .map(|(_, _, _, value)| value)
    .collect()
}

fn enum_error_message(kind: &str, value: &str, candidates: &[&str]) -> String {
  let mut closest = closest_matches(value, candidates, 3);
  let normalized = normalize_token(value);
  if kind == "button" {
    match normalized.as_str() {
      "left" => closest.insert(0, "primary"),
      "right" => closest.insert(0, "secondary"),
      "middle" => closest.insert(0, "auxiliary"),
      _ => {}
    }
    closest.dedup();
    closest.truncate(3);
  }

  format!(
    "Invalid {} '{}'. Closest matches: {}. Supported values: {}.",
    kind,
    value,
    closest.join(", "),
    candidates.join(", ")
  )
}

impl<'de> Deserialize<'de> for InjectMouseButton {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    const CANDIDATES: &[&str] = &["primary", "secondary", "auxiliary", "fourth", "fifth"];
    let raw = String::deserialize(deserializer)?;
    let normalized = normalize_token(&raw);
    match normalized.as_str() {
      "primary" => Ok(Self::Primary),
      "secondary" => Ok(Self::Secondary),
      "auxiliary" => Ok(Self::Auxiliary),
      "fourth" => Ok(Self::Fourth),
      "fifth" => Ok(Self::Fifth),
      _ => Err(D::Error::custom(enum_error_message("button", &raw, CANDIDATES))),
    }
  }
}

impl<'de> Deserialize<'de> for InjectElementState {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    const CANDIDATES: &[&str] = &["pressed", "released"];
    let raw = String::deserialize(deserializer)?;
    let normalized = normalize_token(&raw);
    match normalized.as_str() {
      "pressed" => Ok(Self::Pressed),
      "released" => Ok(Self::Released),
      _ => Err(D::Error::custom(enum_error_message("state", &raw, CANDIDATES))),
    }
  }
}

impl<'de> Deserialize<'de> for InjectKeyLocation {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    const CANDIDATES: &[&str] = &["standard", "left", "right", "numpad"];
    let raw = String::deserialize(deserializer)?;
    let normalized = normalize_token(&raw);
    match normalized.as_str() {
      "standard" => Ok(Self::Standard),
      "left" => Ok(Self::Left),
      "right" => Ok(Self::Right),
      "numpad" => Ok(Self::Numpad),
      _ => Err(D::Error::custom(enum_error_message("location", &raw, CANDIDATES))),
    }
  }
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
  InjectEvents {
    window_id: Option<WindowId>,
    events: Vec<InjectedUiEvent>,
    reply: tokio::sync::oneshot::Sender<Result<InjectEventsResult, String>>,
  },
}
