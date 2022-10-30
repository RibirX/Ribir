use crate::{impl_query_self_only, prelude::*};

#[derive(SingleChild, Declare, Clone)]
pub struct Offstage {
  pub offstage: bool,
}

impl Render for Offstage {
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    if !self.offstage {
      ctx
        .single_child()
        .map_or_else(Size::zero, |c| ctx.perform_child_layout(c, clamp))
    } else {
      Size::zero()
    }
  }

  #[inline]
  fn paint(&self, _: &mut PaintingCtx) {}

  fn hit_test(&self, _: &TreeCtx, _: Point) -> HitTest {
    HitTest {
      hit: false,
      can_hit_child: !self.offstage,
    }
  }
}

impl Query for Offstage {
  impl_query_self_only!();
}

impl Offstage {
  #[inline]
  pub fn new(offstage: bool) -> Self { Self { offstage } }
}
