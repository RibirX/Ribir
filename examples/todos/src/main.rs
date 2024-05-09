use todos::run;

fn main() { run(); }

#[cfg(test)]
use ::todos::todos;
#[cfg(test)]
use ribir::{
  core::test_helper::*,
  material as ribir_material,
  prelude::{AppCtx, Size},
};
#[cfg(test)]
use ribir_dev_helper::*;

#[cfg(test)]
widget_image_test!(todos, wnd_size = Size::new(400., 640.), comparison = 0.002);
