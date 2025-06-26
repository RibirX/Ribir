mod storybook;
use ribir::prelude::*;
pub use storybook::storybook;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run(storybook)
    .with_app_theme(material::purple::light)
    .with_title("Storybook")
    .with_size(Size::new(1024., 768.));
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    storybook,
    WidgetTester::new(storybook)
      .with_wnd_size(Size::new(1024., 768.))
      .with_comparison(0.001)
  );
}
