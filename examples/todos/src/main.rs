mod todos;
use ribir::prelude::*;
mod ui;
use ui::todos;

fn main() {
  unsafe {
    AppCtx::set_app_theme(material::purple::light());
  }

  App::new_window(todos(), Some(Size::new(400., 640.))).set_title("Todos");
  App::exec();
}

#[cfg(test)]
use ribir::core::test_helper::*;
#[cfg(test)]
use ribir::material as ribir_material;
#[cfg(test)]
use ribir_dev_helper::*;
#[cfg(test)]
widget_image_test!(todos, wnd_size = Size::new(400., 640.));
