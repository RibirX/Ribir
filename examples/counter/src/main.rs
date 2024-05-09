mod counter;
use counter::counter;
use ribir::prelude::*;

fn main() {
  App::run(counter())
    .with_title("Counter")
    .with_app_theme(material::purple::light());
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(counter, wnd_size = Size::new(400., 600.), comparison = 0.001);
