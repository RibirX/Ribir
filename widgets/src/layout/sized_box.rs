use ribir_core::prelude::*;

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(SingleChild, Query, Declare, Clone)]
pub struct SizedBox {
  /// The specified size of the box.
  pub size: Size,
}

impl Render for SizedBox {
  #[inline]
  fn perform_layout(&self, _: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.perform_single_child_layout(BoxClamp { min: self.size, max: self.size });
    self.size
  }
  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  fn fix_size() -> impl WidgetBuilder {
    let size: Size = Size::new(100., 100.);
    fn_widget! {
      @SizedBox {
        size,
        @Text { text: "" }
      }
    }
  }
  widget_layout_test!(fix_size, width == 100., height == 100.,);

  fn shrink_size() -> impl WidgetBuilder {
    fn_widget! {
      @SizedBox {
        size: ZERO_SIZE,
        @Text { text: "" }
      }
    }
  }
  widget_layout_test!(
    shrink_size,
    { path = [0], size == ZERO_SIZE,}
    { path = [0, 0], size == ZERO_SIZE,}
  );

  fn expanded_size() -> impl WidgetBuilder {
    fn_widget! {
      @SizedBox {
        size: INFINITY_SIZE,
        @Text { text: "" }
      }
    }
  }
  widget_layout_test!(
    expanded_size,
    wnd_size = Size::new(500., 500.),
    { path = [0], size == Size::new(500., 500.),}
    { path = [0, 0], size == INFINITY_SIZE,}
  );

  fn empty_box() -> impl WidgetBuilder { fn_widget!(SizedBox { size: Size::new(10., 10.) }) }
  widget_layout_test!(empty_box, width == 10., height == 10.,);
}
