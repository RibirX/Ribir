#![feature(specialization, test, decl_macro, raw, negative_impls)]

#[macro_use]
extern crate bitflags;

mod application;
mod render;
mod render_ctx;
mod render_object;
mod render_object_box;
mod util;
pub mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::render::*;
  pub use crate::widget::*;
  pub use canvas::{Point, Rect, Size};
}

#[cfg(test)]
pub mod test;
