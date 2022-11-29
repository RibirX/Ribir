use crate::{impl_query_self_only, prelude::*};

#[derive(SingleChild, Declare, Clone)]
pub struct Visibility {
  #[declare(builtin)]
  pub visible: bool,
}

impl Render for Visibility {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
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
