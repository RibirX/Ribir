mod todos;
use ribir::prelude::*;
mod ui;
use ui::todos;

fn main() {
  App::run(todos())
    .set_app_theme(material::purple::light())
    .on_window(|wnd| {
      wnd.set_title("Todos");
    });
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(todos, wnd_size = Size::new(400., 640.));
