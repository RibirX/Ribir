mod ui;
mod wordle;
use ribir::prelude::*;
pub use ui::wordle_game;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run(wordle_game)
    .with_app_theme(material::purple::light())
    .with_size(Size::new(700., 620.));
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    wordle_game,
    WidgetTester::new(wordle_game)
      .with_wnd_size(Size::new(700., 620.))
      .with_comparison(0.008)
  );
}
