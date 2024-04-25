mod counter;
use counter::counter;
use ribir::prelude::*;

fn main() {
  App::run(counter())
    .set_app_theme(material::purple::light())
    .on_window(|wnd| {
      wnd.set_title("Counter");
    });
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(counter, wnd_size = Size::new(400., 600.),);
