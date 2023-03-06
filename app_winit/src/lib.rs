#![feature(test, decl_macro, box_into_inner)]

mod event_dispatcher;
mod from_cursor_icon;
mod from_device_id;
mod from_modifiers;
mod from_mouse;
mod from_virtual_key_code;

mod application;
pub mod window;
pub mod prelude {
  #[doc(no_inline)]
  pub use crate::application::Application;
  #[doc(no_inline)]
  pub use crate::window::{Window, WindowBuilder};
  pub use log;
  #[doc(no_inline)]
  pub use ribir_core::prelude::*;
  #[doc(no_inline)]
  pub use ribir_painter::*;
}

