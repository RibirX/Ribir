use wordle_game::run;

fn main() { run() }

#[cfg(test)]
use ribir::{
  core::test_helper::*,
  material as ribir_material,
  prelude::{AppCtx, Size},
};
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
use wordle_game::wordle_game;
#[cfg(test)]
widget_image_test!(wordle_game, wnd_size = Size::new(700., 620.), comparison = 0.008);
