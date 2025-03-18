use crate::prelude::*;

/// Widget with fixed size as a container for its child.
#[derive(Declare, SingleChild)]
pub struct Container {
  pub size: Size,
}

impl Render for Container {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let size = clamp.clamp(self.size);
    ctx.perform_single_child_layout(BoxClamp::max_size(size));
    size
  }

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
  use ribir_dev_helper::*;

  use super::*;
  use crate::test_helper::*;

  const SIZE: Size = Size::new(100., 100.);

  widget_layout_test!(
    smoke,
    WidgetTester::new(fn_widget! { @Container { size: SIZE }}),
    LayoutCase::default().with_size(SIZE)
  );
}
