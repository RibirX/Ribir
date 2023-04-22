pub use ribir_core as core;
pub use ribir_widgets as widgets;

pub mod app;
mod winit_shell_wnd;

pub mod prelude {
  pub use crate::app::*;
  pub use ribir_core::prelude::*;
  pub use ribir_widgets::prelude::*;
}
