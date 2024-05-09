use ::storybook::run;

fn main() { run() }

#[cfg(test)]
use ::storybook::storybook;
#[cfg(test)]
use ribir::{
  core::test_helper::*,
  material as ribir_material,
  prelude::{AppCtx, Size},
};
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(storybook, wnd_size = Size::new(1024., 768.), comparison = 0.0025);
