mod counter;
pub use counter::counter;
use ribir::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  App::run(counter())
    .with_app_theme(material::purple::light())
    .with_size(Size::new(300., 150.))
    .with_title("Counter ribir");
}
