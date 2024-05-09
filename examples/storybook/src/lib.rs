mod storybook;
use ribir::prelude::*;
pub use storybook::storybook;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  App::run(storybook())
    .with_app_theme(material::purple::light())
    .with_title("Storybook")
    .with_size(Size::new(1024., 768.));
}
