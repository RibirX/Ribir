pub mod pomodoro;
pub mod styles;
mod widgets;

use std::{env, path::PathBuf, sync::LazyLock};

use ribir::prelude::*;

fn get_resource_path(resource: &str) -> PathBuf {
  // Try relative to current working directory
  let path = PathBuf::from(resource);
  if path.exists() {
    return path;
  }
  // Try using CARGO_MANIFEST_DIR if available
  if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
    let path = PathBuf::from(manifest_dir).join(resource);
    if path.exists() {
      return path;
    }
  }

  // For tests, try to find resources relative to the examples/pomodoro directory
  let alt_path = PathBuf::from("examples/pomodoro").join(resource);
  if alt_path.exists() {
    return alt_path;
  }

  // If none of the common paths work, return the resource path directly for error
  PathBuf::from(resource)
}

pub static APP_ICON: LazyLock<Resource<PixelImage>> = LazyLock::new(|| {
  let path = get_resource_path("static/icon.png");
  Resource::new(PixelImage::from_png(std::fs::read(path).unwrap().as_slice()))
});

pub fn load_icons() {
  svg_registry::register(
    "pause",
    Svg::open(get_resource_path("static/pause.svg"), true, true).unwrap(),
  );
  svg_registry::register(
    "play",
    Svg::open(get_resource_path("static/play.svg"), true, true).unwrap(),
  );
  svg_registry::register(
    "volume_off",
    Svg::open(get_resource_path("static/volume_off.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "volume_up",
    Svg::open(get_resource_path("static/volume_up.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "skip_next",
    Svg::open(get_resource_path("static/skip_next.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "close",
    Svg::open(get_resource_path("static/close.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "minimize",
    Svg::open(get_resource_path("static/minimize.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "full",
    Svg::open(get_resource_path("static/full.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "mini",
    Svg::open(get_resource_path("static/mini.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "pin",
    Svg::open(get_resource_path("static/pin.svg"), true, false).unwrap(),
  );
  svg_registry::register(
    "pin_off",
    Svg::open(get_resource_path("static/pin_off.svg"), true, false).unwrap(),
  );
}

pub(crate) struct UiState {
  pub(crate) current_page: PomodoroPage,
  pub(crate) keep_on_top: bool,
}

impl UiState {
  pub(crate) fn in_mini(&self) -> bool { self.current_page == PomodoroPage::Mini }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PomodoroPage {
  Main,
  Mini,
  Setting,
}

impl Default for UiState {
  fn default() -> Self { Self { current_page: PomodoroPage::Main, keep_on_top: false } }
}
