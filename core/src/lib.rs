#![feature(specialization, test, decl_macro, raw)]

#[macro_use]
extern crate bitflags;

mod application;
mod render;
mod util;
mod widget;
pub mod prelude {
  pub use crate::application::Application;
  pub use crate::render::*;
  pub use crate::widget::*;
}

#[cfg(test)]
pub mod test;
