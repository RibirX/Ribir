use crate::{impl_query_self_only, prelude::*};

/// A widget that overlap children align with left top.
#[derive(MultiChild, Declare)]
pub struct Stack;

impl Render for Stack {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let (ctx, children) = ctx.split_children();
    children.fold(Size::zero(), |size, c| {
      let child_size = ctx.perform_child_layout(c, clamp);
      size.max(child_size)
    })
  }

  fn paint(&self, _: &mut PaintingCtx) {
    // nothing to paint.
  }
}

impl Query for Stack {
  impl_query_self_only!();
}

#[cfg(test)]
mod tests {
  use crate::test::widget_and_its_children_box_rect;

  use super::*;
  #[test]
  fn smoke() {
    let one = Size::new(1., 1.);
    let five = Size::new(5., 5.);
    let w = widget! {
      Stack {
        SizedBox { size: one}
        SizedBox { size: five}
      }
    };
    let (rect, _) = widget_and_its_children_box_rect(w, Size::new(100., 100.));
    assert_eq!(rect.size, five)
  }
}
