mod todos;
use ribir::prelude::*;
mod ui;
use ui::todos;

fn main() {
  App::run(todos())
    .with_app_theme(material::purple::light())
    .with_title("Todos");
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(todos, wnd_size = Size::new(400., 640.), comparison = 0.002);
