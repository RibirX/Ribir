use crate::{impl_query_self_only, prelude::*};

#[derive(Declare, SingleChild, Clone)]
pub struct IgnorePointer {
  pub ignore: bool,
}

impl Render for IgnorePointer {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  fn can_overflow(&self) -> bool { true }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest {
      hit: false,
      can_hit_child: !self.ignore,
    }
  }
}

impl Query for IgnorePointer {
  impl_query_self_only!();
}

impl IgnorePointer {
  #[inline]
  pub fn new(ignore: bool) -> Self { Self { ignore } }
}
