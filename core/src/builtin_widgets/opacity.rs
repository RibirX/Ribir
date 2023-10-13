use crate::prelude::*;

#[derive(Declare, Default, Query, Clone, SingleChild)]
pub struct Opacity {
  #[declare(builtin, default = 1.)]
  pub opacity: f32,
}

impl Render for Opacity {
  #[inline]
  fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
    ctx.assert_perform_single_child_layout(clamp)
  }

  fn paint(&self, ctx: &mut PaintingCtx) { ctx.painter().apply_alpha(self.opacity); }

  fn only_sized_by_parent(&self) -> bool { false }

  fn hit_test(&self, _: &HitTestCtx, _: Point) -> HitTest {
    HitTest { hit: false, can_hit_child: true }
  }
}
