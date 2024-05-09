use ::messages::run;

fn main() { run() }

#[cfg(test)]
use ::messages::messages;
#[cfg(test)]
use ribir::{
  core::test_helper::*,
  material as ribir_material,
  prelude::{AppCtx, Size},
};
#[cfg(test)]
use ribir_dev_helper::*;

#[cfg(test)]
widget_image_test!(messages, wnd_size = Size::new(400., 600.), comparison = 0.004);
