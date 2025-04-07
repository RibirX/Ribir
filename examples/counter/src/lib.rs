use ribir::prelude::*;

pub fn counter() -> Widget<'static> {
  let cnt = Stateful::new(0);
  button! {
    h_align: HAlign::Center,
    v_align: VAlign::Center,
    on_tap: move |_| *$cnt.write() += 1,
    @pipe!($cnt.to_string())
  }
  .into_widget()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
  #[cfg(target_arch = "wasm32")]
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  App::run(list! {
    select_mode: ListSelectMode::Single,
    @ListItem {
      @Icon { @named_svgs::default() }
      @ListItemHeadline { @ { "Icon"} }
      @ListItemSupporting { @ { "description"} }
      @ListItemTrailingSupporting { @ { "100+"} }
    }
    @ListItem {
      disabled: true,
      @Icon { @named_svgs::default() }
      @ListItemHeadline { @ { "Only Headline"} }
      @Trailing { @Icon { @named_svgs::default() } }
    }
    @ListCustomItem { @Text { text: "Custom Item" } }
    @ListItem {
      @Icon { @named_svgs::default() }
      @ListItemHeadline { @ { "Only Headline"} }
      @ListItemTrailingSupporting { @ { "100+"} }
      @Trailing { @Icon { @named_svgs::default() } }
    }
    @ListItem {
      @Avatar { @ { "A" } }
      @ListItemHeadline { @ { "Avatar"} }
      @ListItemSupporting { @ { "description"} }
      @Trailing { @Icon { @named_svgs::default() } }
    }
    @ListItem {
      @ListItemImg {
        @Container { size: Size::new(100., 100.), background: Color::PINK }
      }
      @ListItemHeadline { @ { "Image Item"} }
      @ListItemSupporting { @ { "description"} }
    }
    @ListItem {
      @ListItemThumbNail {
        @Container { size: Size::new(160., 90.), background: Color::GREEN }
      }
      @ListItemHeadline { @ { "Counter"} }
      @ListItemSupporting {
        lines: 2usize,
        @ { "there is supporting lines, many lines, wrap to multiple lines, xxhadkasda"}
      }
      @ListItemTrailingSupporting { @ { "100+" } }
      @Trailing { @Icon { @named_svgs::default() } }
    }
  })
  .with_app_theme(material::purple::light())
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
    WidgetTester::new(counter)
      .with_wnd_size(Size::new(320., 240.))
      .with_comparison(0.0001)
  );
}
