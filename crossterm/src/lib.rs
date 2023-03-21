#![feature(test, decl_macro, box_into_inner)]

// mod from_cursor_icon;
mod from_device_id;
mod from_element_state;
mod from_event;
mod from_keyboard_input;
mod from_modifiers;
mod from_mouse;
// mod from_mouse_scroll_delta;
// mod from_size;
// mod from_touch_phase;
mod from_virtual_key_code;
mod from_window;
mod from_window_id;
mod shell_window;
mod window_builder;

pub mod prelude {
  #[doc(no_inline)]
  // #[doc(no_inline)]
  // pub use crate::from_cursor_icon::*;
  #[doc(no_inline)]
  pub use crate::from_device_id::*;
  #[doc(no_inline)]
  pub use crate::from_element_state::*;
  #[doc(no_inline)]
  pub use crate::from_event::*;
  #[doc(no_inline)]
  pub use crate::from_keyboard_input::*;
  #[doc(no_inline)]
  pub use crate::from_modifiers::*;
  #[doc(no_inline)]
  pub use crate::from_mouse::*;
  // #[doc(no_inline)]
  // pub use crate::from_mouse_scroll_delta::*;
  // #[doc(no_inline)]
  // pub use crate::from_size::*;
  // #[doc(no_inline)]
  // pub use crate::from_touch_phase::*;
  #[doc(no_inline)]
  pub use crate::from_virtual_key_code::*;
  #[doc(no_inline)]
  pub use crate::from_window::*;
  #[doc(no_inline)]
  pub use crate::from_window_id::*;
  #[doc(no_inline)]
  pub use crate::shell_window::*;
  #[doc(no_inline)]
  pub use crate::window_builder::*;
}
