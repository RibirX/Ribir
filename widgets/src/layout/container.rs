use ribir_core::prelude::*;

/// Widget with fixed size as a container for its child.
#[derive(Declare, SingleChild)]
pub struct Container {
  pub size: Size,
}

impl Render for Container {
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if let Some(mut l) = ctx.single_child_layouter() {
      clamp.max = clamp.max.min(self.size);
      clamp.min = clamp.max.min(clamp.min);
      l.perform_widget_layout(clamp);
    };
    self.size
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  #[inline]
  fn only_sized_by_parent(&self) -> bool { true }
}

impl Query for Container {
  ribir_core::impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use ribir_core::test_helper::*;
  use ribir_dev_helper::*;
  use ribir_geom::Size;

  const SIZE: Size = Size::new(100., 100.);
  fn smoke() -> Widget {
    widget! { Container { size: SIZE }}
  }
  widget_layout_test!(smoke, size == SIZE,);
}
