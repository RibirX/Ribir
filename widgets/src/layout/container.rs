use ribir_core::prelude::*;

/// Widget with fixed size as a container for its child.
#[derive(Declare, SingleChild)]
pub struct Container {
  pub size: Size,
}

impl Render for Container {
  fn perform_layout(&self, mut clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let size = self.size;
    if let Some(c) = ctx.single_child() {
      clamp.max = clamp.max.min(size);
      clamp.min = clamp.max.min(clamp.min);
      ctx.perform_child_layout(c, clamp);
    }
    size
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
  use ribir_core::test::widget_and_its_children_box_rect;

  #[test]
  fn smoke() {
    let size = Size::new(100., 100.);
    let w = widget! { Container { size }};
    let (rect, _) = widget_and_its_children_box_rect(w, Size::new(200., 200.));
    assert_eq!(rect, Rect::from_size(size));
  }
}
