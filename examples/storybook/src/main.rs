mod storybook;
use ribir::prelude::*;
use storybook::storybook;

fn main() {
  unsafe {
    AppCtx::set_app_theme(material::purple::light());
  }

  App::new_window(storybook(), Some(Size::new(1024., 768.))).set_title("Storybook");
  App::exec();
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(storybook, wnd_size = Size::new(1024., 768.),);
