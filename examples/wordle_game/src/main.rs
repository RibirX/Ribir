use ribir::prelude::*;

use crate::ui::wordle_game;
mod ui;
mod wordle;

fn main() {
  App::run(wordle_game())
    .set_app_theme(material::purple::light())
    .on_window(|wnd| {
      wnd.set_title("Wordle Game");
    });
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(wordle_game, wnd_size = Size::new(700., 620.),);
