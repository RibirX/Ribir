use ribir_core::prelude::*;

/// A box with a specified size.
///
/// This widget forces its child to have a specific width and/or height
/// (assuming values are permitted by the parent of this widget).
#[derive(SingleChild, Declare, Clone)]
pub struct SizedBox {
  /// The specified size of the box.
  pub size: Size,
}

impl Render for SizedBox {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let size = clamp.clamp(self.size);
    ctx.perform_single_child_layout(BoxClamp { min: size, max: size });
    size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;

  use super::*;
  use crate::prelude::*;

  widget_layout_test!(
    fix_size,
    WidgetTester::new(fn_widget! {
      let size: Size = Size::new(100., 100.);
      @SizedBox {
        size,
        @Text { text: "" }
      }
    }),
    LayoutCase::default().with_size(Size::new(100., 100.))
  );

  widget_layout_test!(
    shrink_size,
    WidgetTester::new(fn_widget! {
      @SizedBox {
        size: ZERO_SIZE,
        @Text { text: "" }
      }
    }),
    LayoutCase::default().with_size(ZERO_SIZE),
    LayoutCase::new(&[0, 0]).with_size(ZERO_SIZE)
  );

  widget_layout_test!(
    expanded_size,
    WidgetTester::new(fn_widget! {
      @SizedBox {
        size: INFINITY_SIZE,
        @Text { text: "" }
      }
    })
    .with_wnd_size(Size::new(500., 500.)),
    LayoutCase::default().with_size(Size::new(500., 500.)),
    LayoutCase::new(&[0, 0]).with_size(Size::new(500., 500.))
  );

  widget_layout_test!(
    empty_box,
    WidgetTester::new(fn_widget!(SizedBox { size: Size::new(10., 10.) })),
    LayoutCase::default().with_size(Size::new(10., 10.))
  );
}
