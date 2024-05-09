mod todos;
use ribir::prelude::*;
mod ui;
pub use ui::todos;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  App::run(todos())
    .with_app_theme(material::purple::light())
    .with_title("Todos");
}
