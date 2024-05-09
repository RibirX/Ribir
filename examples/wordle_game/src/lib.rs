mod ui;
mod wordle;
use ribir::{app::App, prelude::*};
pub use ui::wordle_game;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  App::run(wordle_game())
    .with_app_theme(material::purple::light())
    .with_size(Size::new(700., 620.));
}
