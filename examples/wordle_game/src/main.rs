use ribir::prelude::*;

use crate::ui::wordle_game;
mod ui;
mod wordle;

fn main() {
  unsafe {
    AppCtx::set_app_theme(material::purple::light());
  }

  App::new_window(wordle_game(), Some(Size::new(700., 620.))).set_title("Messages");
  App::exec();
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(wordle_game, wnd_size = Size::new(700., 620.),);
