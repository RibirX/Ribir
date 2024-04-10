use crate::prelude::*;

#[derive(Declare, SingleChild, Query, Clone)]
pub struct IgnorePointer {
  #[declare(default = true)]
  pub ignore: bool,
}

impl Render for IgnorePointer {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: !self.ignore }
  }
}

impl IgnorePointer {
  #[inline]
  pub fn new(ignore: bool) -> Self { Self { ignore } }
}
