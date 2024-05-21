mod todos;
use ribir::prelude::*;
mod ui;
pub use ui::todos;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run(todos())
    .with_app_theme(material::purple::light())
    .with_title("Todos");
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_test!(todos, wnd_size = Size::new(400., 640.), comparison = 0.002);
}
