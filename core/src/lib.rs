#![allow(incomplete_features)]
#![feature(specialization, test, decl_macro, negative_impls, drain_filter)]
#[macro_use]
extern crate bitflags;

extern crate widget_derive;

mod application;
mod render;
mod util;
pub mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::render::*;
  pub use crate::widget;
  pub use crate::widget::{build_ctx::BuildCtx, widget_tree::WidgetId, *};
  pub use canvas::*;
  pub use rxrust::prelude::*;
  pub use widget_derive::Widget;
}

#[cfg(test)]
pub mod test;
