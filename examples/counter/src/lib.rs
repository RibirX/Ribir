use ribir::prelude::*;

pub fn counter(ctx: &mut BuildCtx) -> Widget<'static> {
  let cnt = Stateful::new(0);
  let f = fn_widget! {
    @Row {
      @FilledButton {
        on_tap: move |_| *$cnt.write() += 1,
        @{ Label::new("Inc") }
      }
      @H1 { text: pipe!($cnt.to_string()) }
    }
  };
  f(ctx)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run(counter)
    .with_app_theme(material::purple::light())
    .with_size(Size::new(300., 150.))
    .with_title("Counter");
}

#[cfg(test)]
mod tests {
  use ribir::{core::test_helper::*, material as ribir_material};
  use ribir_dev_helper::*;

  use super::*;

  widget_image_test!(counter, counter, wnd_size = Size::new(400., 600.), comparison = 0.001);
}
