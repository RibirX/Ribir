mod storybook;
use ribir::prelude::*;
use storybook::storybook;

fn main() {
  App::run(storybook())
    .with_app_theme(material::purple::light())
    .with_title("Storybook")
    .with_size(Size::new(1024., 768.));
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(storybook, wnd_size = Size::new(1024., 768.),);
