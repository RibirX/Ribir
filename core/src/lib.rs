#![feature(specialization, test, decl_macro)]

mod application;
mod render_ctx;
mod render_object;
mod render_object_box;
mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::render_object::RenderObject;
  pub use crate::widget::*;
}
