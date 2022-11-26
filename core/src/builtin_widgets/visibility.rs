use crate::{impl_query_self_only, prelude::*};

#[derive(SingleChild, Declare, Clone)]
pub struct Visibility {
  #[declare(builtin)]
  pub visible: bool,
}

impl Render for Visibility {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if self.visible {
      ctx
        .single_child()
        .map_or_else(Size::zero, |c| ctx.perform_child_layout(c, clamp))
    } else {
      Size::zero()
    }
  }

  #[inline]
  fn paint(&self, ctx: &mut PaintingCtx) {
    if !self.visible {
      ctx.painter().apply_alpha(0.);
    }
  }

  fn can_overflow(&self) -> bool { self.visible }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest {
      hit: false,
      can_hit_child: self.visible,
    }
  }
}

impl Query for Visibility {
  impl_query_self_only!();
}

impl Visibility {
  #[inline]
  pub fn new(visible: bool) -> Self { Self { visible } }
}
