use crate::prelude::*;

#[derive(Declare, SingleChild)]
/// A widget that imposes no constraints on its child, allowing it to layout and
/// display as its "natural" size. Its size is equal to its child.
pub struct UnconstrainedBox;

impl Render for UnconstrainedBox {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map_or_else(Size::zero, |c| ctx.perform_child_layout(c, clamp.expand()))
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}
}

impl Query for UnconstrainedBox {
  crate::impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test::widget_and_its_children_box_rect;

  #[test]
  fn smoke() {
    let size = Size::new(200., 200.);
    let w = widget! {
      UnconstrainedBox {
        SizedBox { size}
      }
    };

    let constrained_size = Size::new(100., 100.);
    let (rect, children) = widget_and_its_children_box_rect(w, constrained_size);
    assert_eq!(rect, Rect::from_size(constrained_size));
    assert_eq!(children[0], Rect::from_size(size));
  }
}
