use ribir::prelude::*;

pub fn counter(cnt: &'static Stateful<i32>) -> Widget<'static> {
  fn_widget! {
    let img = Resource::new(PixelImage::from_png(include_bytes!("../../attachments/3DDD-2.png")));
    @Container {
      size: Size::new(100., 100.),
      filter: Filter::blur(3.),
      @Container {
        size: Size::new(40., 40.),
        background: Color::GREEN,
      }
    }
  }
  .into_widget()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run_with_data(|| Stateful::new(0), counter)
    .with_app_theme(material::purple::light)
    .with_size(Size::new(320., 240.))
    .with_title("Counter");
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_tests!(
    counter,
    WidgetTester::new_with_data(Stateful::new(0), counter)
      .with_wnd_size(Size::new(320., 240.))
      .with_comparison(0.0001)
  );
}
