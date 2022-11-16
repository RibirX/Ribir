use ribir_core::{impl_query_self_only, prelude::*};

/// A widget that overlap children align with left top.
#[derive(MultiChild, Declare)]
pub struct Stack {
  #[declare(default)]
  fit: StackFit,
}

/// How to size the non-positioned children of a [Stack]. (same as flutter)
#[derive(Default)]
pub enum StackFit {
  /// The constraints passed to the stack from its parent are loosened.
  ///
  /// For example, if the stack has constraints that force it to 350x600, then
  /// this would allow the non-positioned children of the stack to have any
  /// width from zero to 350 and any height from zero to 600.
  ///
  /// See also:
  ///
  ///  * [Center], which loosens the constraints passed to its child and then
  ///    centers the child in itself.
  ///  * [BoxConstraints.loosen], which implements the loosening of box
  ///    constraints.
  #[default]
  Loose,

  /// The constraints passed to the stack from its parent are tightened to the
  /// biggest size allowed.
  ///
  /// For example, if the stack has loose constraints with a width in the range
  /// 10 to 100 and a height in the range 0 to 600, then the non-positioned
  /// children of the stack would all be sized as 100 pixels wide and 600 high.
  Expand,

  /// The constraints passed to the stack from its parent are passed unmodified
  /// to the non-positioned children.
  ///
  /// For example, if a [Stack] is an [Expanded] child of a [Row], the
  /// horizontal constraints will be tight and the vertical constraints will be
  /// loose.
  Passthrough,
}

impl Render for Stack {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    let clamp = match self.fit {
      StackFit::Loose => clamp.loose(),
      StackFit::Expand => BoxClamp { min: clamp.max, max: clamp.max },
      StackFit::Passthrough => clamp,
    };

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
  use crate::prelude::*;
  use ribir_core::test::widget_and_its_children_box_rect;

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
