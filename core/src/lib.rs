#![feature(min_specialization, test, decl_macro, negative_impls)]

#[macro_use]
extern crate bitflags;

mod application;
pub mod events;
mod render;
mod util;
pub mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::events::Event;
  pub use crate::render::*;
  pub use crate::widget::build_ctx::BuildCtx;
  pub use crate::widget::widget_tree::WidgetId;
  pub use crate::widget::*;
  pub use canvas::{DevicePoint, DeviceRect, DeviceSize, Point, Rect, Size};
}

#[cfg(test)]
pub mod test;
