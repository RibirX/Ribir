use counter::run;

fn main() { run(); }

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn bin() {}

#[cfg(test)]
use counter::counter;
#[cfg(test)]
use ribir::{
  core::test_helper::*,
  material as ribir_material,
  prelude::{AppCtx, Size},
};
#[cfg(test)]
use ribir_dev_helper::*;

#[cfg(test)]
widget_image_test!(counter, wnd_size = Size::new(400., 600.), comparison = 0.001);
