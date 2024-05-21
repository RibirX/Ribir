mod messages;
pub use messages::messages;
use ribir::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run(messages())
    .with_app_theme(material::purple::light())
    .with_size(Size::new(400., 600.))
    .with_title("Messages");
}
