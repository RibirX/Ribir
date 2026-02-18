#![allow(clippy::needless_lifetimes)]
#[macro_use]
extern crate bitflags;

pub mod animation;
pub mod builtin_widgets;
pub mod clipboard;
mod context;
pub mod data_widget;
pub mod declare;
pub mod events;
pub mod local_sender;
pub mod pipe;
pub(crate) mod render_helper;
mod state;
pub mod ticker;
pub mod widget;
pub mod widget_children;
pub(crate) mod widget_tree;
pub mod window;
pub use rxrust;
pub mod convert;
pub mod event_loop;
pub mod overlay;
pub mod query;
pub mod reusable;

pub mod wrap_render;

/// Represents measurement units for positioning and sizing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Measure {
  /// Value in logical pixels.
  Pixel(f32),

  /// The value represents a fraction of the maximum size provided by the
  /// finite parent clamp, corresponding to the parent's size if the parent is a
  /// fixed-size container. A value of 1.0 corresponds to 100%.
  Unit(f32),
}

pub mod prelude {
  pub use ribir_algo::*;
  pub use ribir_geom::*;
  pub use ribir_macros::*;
  pub use ribir_painter::*;
  #[doc(hidden)]
  pub use rxrust::prelude::*;
  pub use smallvec;
  pub use tracing;

  pub use super::{
    Measure, MeasureExt,
    animation::*,
    builtin_widgets::*,
    class_chain_impl, class_names,
    context::*,
    convert::*,
    declare::*,
    event_loop::EventLoop,
    events::*,
    overlay::{AutoClosePolicy, Overlay, OverlayStyle},
    pipe::Pipe,
    providers,
    query::*,
    reusable::*,
    state::*,
    style_class,
    ticker::{Duration, Instant},
    widget::*,
    widget_children::*,
    widget_tree::{Anchor, AnchorX, AnchorY, BoxClamp, DirtyPhase, LayoutInfo, TrackId, WidgetId},
    window::{Window, WindowLevel},
  };
  pub use crate::*;
}

#[cfg(feature = "test-utils")]
pub mod test_helper;

#[cfg(feature = "debug")]
pub mod debug_tool;

#[cfg(feature = "debug")]
pub mod logging;

impl<T> From<T> for Measure
where
  T: Into<f32>,
{
  fn from(value: T) -> Self { Measure::Pixel(value.into()) }
}

impl Default for Measure {
  fn default() -> Self { Measure::Pixel(0.0) }
}

impl Measure {
  pub fn into_pixel(self, max_clamp: f32) -> f32 {
    match self {
      Measure::Pixel(x) => x,
      Measure::Unit(x) => {
        if x.is_finite() {
          x * max_clamp
        } else {
          0.
        }
      }
    }
  }
}

/// Extension trait for convenient Measure construction
pub trait MeasureExt {
  fn px(self) -> Measure;
  fn unit(self) -> Measure;
  fn percent(self) -> Measure;
}

impl MeasureExt for f32 {
  fn px(self) -> Measure { Measure::Pixel(self) }
  fn unit(self) -> Measure { Measure::Unit(self) }
  fn percent(self) -> Measure { Measure::Unit(self / 100.) }
}

impl MeasureExt for f64 {
  fn px(self) -> Measure { Measure::Pixel(self as f32) }
  fn unit(self) -> Measure { Measure::Unit(self as f32) }
  fn percent(self) -> Measure { Measure::Unit(self as f32 / 100.) }
}

impl MeasureExt for i32 {
  fn px(self) -> Measure { Measure::Pixel(self as f32) }
  fn unit(self) -> Measure { Measure::Unit(self as f32) }
  fn percent(self) -> Measure { Measure::Unit(self as f32 / 100.) }
}
