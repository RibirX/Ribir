#[cfg(all(feature = "crossterm", not(feature = "winit")))]
extern crate ribir_crossterm as ribir_platform;

#[cfg(all(feature = "winit", not(feature = "crossterm")))]
extern crate ribir_winit as ribir_platform;

#[cfg(all(feature = "winit", feature = "crossterm"))]
compile_error!("feature \"winit\" and feature \"crossterm\" cannot be enabled at the same time");

pub mod application;
pub mod prelude {
  pub use crate::application::*;
  pub use ribir_core::prelude::*;
  #[cfg(any(feature = "crossterm", feature = "winit"))]
  pub use ribir_platform::prelude::*;
}
