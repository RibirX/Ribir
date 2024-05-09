mod messages;
use messages::messages;
use ribir::prelude::*;

fn main() {
  App::run(messages())
    .with_app_theme(material::purple::light())
    .with_title("Messages");
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(messages, wnd_size = Size::new(400., 600.), comparison = 0.004);
