#![feature(raw, specialization)]

mod application;
mod render_object;
mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::render_object::RenderObject;
  pub use crate::widget::*;
}
