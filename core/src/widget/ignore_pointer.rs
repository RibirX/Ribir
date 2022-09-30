use crate::{impl_query_self_only, prelude::*};

#[derive(Declare, SingleChild, Clone)]
pub struct IgnorePointer {
  pub ignore: bool,
}

impl Render for IgnorePointer {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx
      .single_child()
      .map_or_else(Size::zero, |c| ctx.perform_child_layout(c, clamp))
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &TreeCtx, _: Point) -> HitTest {
    HitTest {
      hit: !self.ignore,
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
