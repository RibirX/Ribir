pub use ribir_core as core;
#[cfg(feature = "widgets")]
pub use ribir_widgets as widgets;
pub mod app;
mod backends;

#[cfg(not(target_arch = "wasm32"))]
pub mod clipboard;
mod winit_shell_wnd;
#[cfg(feature = "material")]
pub use ribir_material as material;

mod platform;
pub use platform::*;
pub mod prelude {
  pub use ribir_core::prelude::*;

  #[cfg(feature = "material")]
  pub use super::material;
  #[cfg(feature = "widgets")]
  pub use super::widgets::prelude::*;
  pub use crate::app::*;
}
